#[macro_use]
extern crate lazy_static;
extern crate evdev_rs as evdev;
extern crate mio;

use evdev::*;
use evdev::enums::*;
use std::io;
use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::os::unix::io::AsRawFd;
use mio::{Poll,Events,Token,Interest};
use mio::unix::SourceFd;
use std::fs;
use props_rs::*;
use libc::time_t;
use std::collections::HashMap;

static PERF_MAX:    EventCode = EventCode::EV_KEY(EV_KEY::BTN_TL2);
static PERF_NORM:   EventCode = EventCode::EV_KEY(EV_KEY::BTN_TL);
static DARK_ON:     EventCode = EventCode::EV_KEY(EV_KEY::BTN_DPAD_LEFT);
static DARK_OFF:    EventCode = EventCode::EV_KEY(EV_KEY::BTN_DPAD_RIGHT);
static WIFI_ON:     EventCode = EventCode::EV_KEY(EV_KEY::BTN_TR);
static WIFI_OFF:    EventCode = EventCode::EV_KEY(EV_KEY::BTN_TR2);
static POWER_OFF:   EventCode = EventCode::EV_KEY(EV_KEY::KEY_POWER);
static MIN_POWERKEY_DELAY: time_t = 1;

lazy_static! {
    static ref DEVICE: &'static str = {
        let lines = fs::read_to_string("/opt/.retrooz/device").expect("Can't read file '/opt/.retrooz/device'.").trim_end_matches(&['\r', '\n'][..]).to_string();
        if lines.is_empty() {
            return "rgb10maxtop";
        }
        Box::leak(lines.into_boxed_str())
    };

    static ref IS_OGA1: bool = {
        if *DEVICE == "oga1" {
            return true;
        }

        false
    };

    static ref HOTKEY: EventCode = {
        if *DEVICE == "rgb10maxtop" {
            return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY4);
        }
        else if *DEVICE == "rgb10maxnative" {
            return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY2);
        }

        //if (DEVICE == "ogs") or (DEVICE == "oga") or (DEVICE == "oga1")
        EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY6)
    };

    static ref BRIGHT_UP: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY5);
        }

        //if (DEVICE == "ogs") or (DEVICE == "oga") or (DEVICE == "rgb10maxtop") or (DEVICE == "rgb10maxnative")
        EventCode::EV_KEY(EV_KEY::BTN_DPAD_UP)
    };

    static ref BRIGHT_DOWN: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY4);
        }

        //if (DEVICE == "ogs") or (DEVICE == "oga") or (DEVICE == "rgb10maxtop") or (DEVICE == "rgb10maxnative")
        EventCode::EV_KEY(EV_KEY::BTN_DPAD_DOWN)
    };

    static ref VOL_UP: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY3);
        }

        //if (DEVICE == "ogs") or (DEVICE == "oga") or (DEVICE == "rgb10maxtop") or (DEVICE == "rgb10maxnative")
        EventCode::EV_KEY(EV_KEY::BTN_NORTH)
    };

    static ref VOL_DOWN: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY2);
        }

        //if (DEVICE == "ogs") or (DEVICE == "oga") or (DEVICE == "rgb10maxtop") or (DEVICE == "rgb10maxnative")
        EventCode::EV_KEY(EV_KEY::BTN_SOUTH)
    };

    static ref MUTE: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_DPAD_DOWN);
        }

        //if (DEVICE == "ogs") or (DEVICE == "oga") or (DEVICE == "rgb10maxtop") or (DEVICE == "rgb10maxnative")
        EventCode::EV_KEY(EV_KEY::BTN_WEST)
    };

    static ref VOL_NORM: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_DPAD_UP);
        }

        //if (DEVICE == "ogs") or (DEVICE == "oga") or (DEVICE == "rgb10maxtop") or (DEVICE == "rgb10maxnative")
        EventCode::EV_KEY(EV_KEY::BTN_EAST)
    };

    static ref SUSPEND: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_NORTH);
        }
        else if *DEVICE == "rgb10maxnative" {
            return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY4);
        }

        //if (DEVICE == "ogs") or (DEVICE == "oga") or (DEVICE == "rgb10maxtop")
        EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY2)
    };

    static ref POWERKEY_PROPERTIES: HashMap<String, String> = {
        let lines = fs::read_to_string("/usr/local/etc/powerkey.conf").expect("Can't read file '/usr/local/etc/powerkey.conf'.");
        let parsed = parse(lines.as_bytes()).expect("Can't parse properties of '/usr/local/etc/powerkey.conf'");
        
        to_map(parsed)
    };

    static ref IS_DOUBLE_PUSH_POWER_OFF_ACTIVE: bool = {       
        println!("POWERKEY_PROPERTIES: {}", POWERKEY_PROPERTIES.len());
        for (key, value) in POWERKEY_PROPERTIES.iter() {
            println!("{} / {}", key, value);
        }

        if !POWERKEY_PROPERTIES.is_empty() {
            match POWERKEY_PROPERTIES.get("two_push_shutdown") {
                Some(x) => {
                    if x == "enabled" {
                        return true;
                    }
                },
                None => return false,
            };
        }

        false
    };

    static ref MAX_POWERKEY_DELAY: time_t = {
        if !POWERKEY_PROPERTIES.is_empty() {
            match POWERKEY_PROPERTIES.get("max_delay_time") {
                Some(x) => return x.parse::<i64>().unwrap(),
                None => return 2,
            };
        }

        2
    };

}

fn blinkon() {
    let output = Command::new("brightnessctl").arg("g").stdout(Stdio::piped()).output().expect("Failed to execute brightnessctl");
    let current = String::from_utf8(output.stdout).unwrap();
    Command::new("brightnessctl").args(&["s","0"]).output().expect("Failed to execute brightnessctl");
    Command::new("sleep").arg("0.2").output().expect("Failed to Sleep");
    Command::new("brightnessctl").args(&["s","160"]).output().expect("Failed to execute brightnessctl");
    Command::new("sleep").arg("0.2").output().expect("Failed to Sleep");
    Command::new("brightnessctl").args(&["s","0"]).output().expect("Failed to execute brightnessctl");
    Command::new("sleep").arg("0.2").output().expect("Failed to Sleep");
    Command::new("brightnessctl").arg("s").arg(current).output().expect("Failed to execute brightnessctl");
}

fn blinkoff() {
    let output = Command::new("brightnessctl").arg("g").stdout(Stdio::piped()).output().expect("Failed to execute brightnessctl");
    let current = String::from_utf8(output.stdout).unwrap();
    Command::new("brightnessctl").args(&["s","0"]).output().expect("Failed to execute brightnessctl");
    Command::new("sleep").arg("0.3").output().expect("Failed to Sleep");
    Command::new("brightnessctl").arg("s").arg(current).output().expect("Failed to execute brightnessctl");
}

fn inc_brightness() {
    Command::new("brightnessctl").args(&["s","+2%"]).output().expect("Failed to execute brightnessctl");
}

fn dec_brightness() {
    Command::new("brightnessctl").args(&["-n","s","2%-"]).output().expect("Failed to execute brightnessctl");
}

fn inc_volume() {
    Command::new("amixer").args(&["-q", "sset", "Playback", "1%+"]).output().expect("Failed to execute amixer");
}

fn dec_volume() {
    Command::new("amixer").args(&["-q", "sset", "Playback", "1%-"]).output().expect("Failed to execute amixer");
}

fn mute_volume() {
    Command::new("amixer").args(&["sset", "Playback", "0"]).output().expect("Failed to execute amixer");
}

fn norm_volume() {
    Command::new("amixer").args(&["sset", "Playback", "180"]).output().expect("Failed to execute amixer");
}

fn perf_max() {
    Command::new("perfmax").arg("none").output().expect("Failed to execute performance");
    blinkon();
}

fn perf_norm() {
    Command::new("perfnorm").arg("none").output().expect("Failed to execute performance");
    blinkoff();
}

fn dark_on() {
    Command::new("brightnessctl").args(&["s","10%"]).output().expect("Failed to execute brightnessctl");
}

fn dark_off() {
    Command::new("brightnessctl").args(&["s","50%"]).output().expect("Failed to execute brightnessctl");
}

fn wifi_on() {
    blinkon();
    Command::new("nmcli").args(&["radio","wifi","on"]).output().expect("Failed to execute wifi on");
}

fn wifi_off() {
    Command::new("nmcli").args(&["radio","wifi","off"]).output().expect("Failed to execute wifi off");
    blinkoff();
}

fn suspend() {
    Command::new("sudo").args(&["systemctl", "suspend"]).output().expect("Failed to execute suspend");
}

fn power_off() {
    Command::new("sudo").args(&["shutdown", "-h", "now"]).output().expect("Failed to execute power off");
}

fn process_event(_dev: &Device, ev: &InputEvent, hotkey: bool, _first_push_power_off: time_t) {
/*
        println!("Event: time {}.{} type {} code {} value {} hotkey {}",
             ev.time.tv_sec,
             ev.time.tv_usec,
             ev.event_type,
             ev.event_code,
             ev.value,
             hotkey);
*/

    if ev.value == 1 {
/*
        println!("Event: time {}.{} type {} code {} value {} hotkey {}",
            ev.time.tv_sec, ev.time.tv_usec, ev.event_type, ev.event_code,
            ev.value, hotkey);
        println!("Device: {}", *DEVICE);
        println!("Is OGA v1.1: {}", *IS_OGA1);
        println!("IS double push power off button active?: {}", *IS_DOUBLE_PUSH_POWER_OFF_ACTIVE);
*/
        if hotkey {
            if !*IS_OGA1 {
                if ev.event_code == *BRIGHT_UP {
                    inc_brightness();
                }
                else if ev.event_code == *BRIGHT_DOWN {
                    dec_brightness();
                }
                else if ev.event_code == *VOL_UP {
                    inc_volume();
                }
                else if ev.event_code == *VOL_DOWN {
                    dec_volume();
                }
            }
            if ev.event_code == *MUTE {
                mute_volume();
            }
            else if ev.event_code == *VOL_NORM {
                norm_volume();
            }
            else if ev.event_code == PERF_MAX {
                perf_max();
            }
            else if ev.event_code == PERF_NORM {
                perf_norm();
            }
            else if ev.event_code == DARK_ON {
                dark_on();
            }
            else if ev.event_code == DARK_OFF {
                dark_off();
            }
            else if ev.event_code == WIFI_ON {
                wifi_on();
            }
            else if ev.event_code == WIFI_OFF {
                wifi_off();
            }
            else if ev.event_code == *SUSPEND {
                suspend();
            }
        }
        else if *IS_OGA1 {
            if ev.event_code == *BRIGHT_DOWN {
                dec_brightness();
            }
            else if ev.event_code == *BRIGHT_UP {
                inc_brightness();
            }
            else if ev.event_code == *VOL_UP {
                inc_volume();
            }
            else if ev.event_code == *VOL_DOWN {
                dec_volume();
            } 
		}
        else if ev.event_code == POWER_OFF && *IS_DOUBLE_PUSH_POWER_OFF_ACTIVE {
            let diff = ev.time.tv_sec - _first_push_power_off;
            //println!("ev.time.tv_sec: {} - _first_push_power_off: {} = {}", ev.time.tv_sec, _first_push_power_off, diff);
            if diff >= MIN_POWERKEY_DELAY && diff < *MAX_POWERKEY_DELAY { // two push in one second
                power_off();
            }
        }
	}
}

fn main() -> io::Result<()> {
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1);
    let mut devs: Vec<Device> = Vec::new();
    let mut hotkey = false;
    let mut first_push_power_off: time_t = 0;

    println!("Device: {}", *DEVICE);
    println!("Is OGA v1.1: {}", *IS_OGA1);
    println!("Is double push power off button active?: {}", *IS_DOUBLE_PUSH_POWER_OFF_ACTIVE);

    let mut i = 0;
    for s in ["/dev/input/event3", "/dev/input/event2", "/dev/input/event0", "/dev/input/event1"].iter() {
        if !Path::new(s).exists() {
            println!("Path {} doesn't exist", s);
            continue;
        }
        let fd = File::open(Path::new(s)).unwrap();
        let mut dev = Device::new().unwrap();
        poll.registry().register(&mut SourceFd(&fd.as_raw_fd()), Token(i), Interest::READABLE)?;
        dev.set_fd(fd)?;
        devs.push(dev);
        println!("Added {}", s);
        i += 1;
    }

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            let dev = &mut devs[event.token().0];
            while dev.has_event_pending() {
                let e = dev.next_event(evdev_rs::ReadFlag::NORMAL);
                match e {
                    Ok(k) => {
                        let ev = &k.1;
                        //println!("Hotkey: {} - {} - {}", *HOTKEY, ev.event_code, hotkey);
                        if ev.event_code == *HOTKEY {
                            hotkey = ev.value == 1;
                            //println!("Hotkey: {} - {}", *HOTKEY, hotkey);
                            //let grab = if hotkey { GrabMode::Grab } else { GrabMode::Ungrab };
                            //dev.grab(grab)?;
                        }
                        process_event(&dev, &ev, hotkey, first_push_power_off);
                        if ev.event_code == POWER_OFF && *IS_DOUBLE_PUSH_POWER_OFF_ACTIVE {
                            first_push_power_off = ev.time.tv_sec;
                            //println!("new first_push_power_off: {}", first_push_power_off);
                        }
                    },
                    _ => ()
                }
            }
        }
    }
}
