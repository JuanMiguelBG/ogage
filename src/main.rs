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

static HOTKEY:      EventCode = EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY6);
static BRIGHT_UP:   EventCode = EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY5);
static BRIGHT_DOWN: EventCode = EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY4);
static VOL_UP:      EventCode = EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY3);
static VOL_DOWN:    EventCode = EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY2);
static MUTE:        EventCode = EventCode::EV_KEY(EV_KEY::BTN_DPAD_DOWN);
static VOL_NORM:    EventCode = EventCode::EV_KEY(EV_KEY::BTN_DPAD_UP);
static PERF_MAX:    EventCode = EventCode::EV_KEY(EV_KEY::BTN_TL2);
static PERF_NORM:   EventCode = EventCode::EV_KEY(EV_KEY::BTN_TL);
static DARK_ON:     EventCode = EventCode::EV_KEY(EV_KEY::BTN_DPAD_LEFT);
static DARK_OFF:    EventCode = EventCode::EV_KEY(EV_KEY::BTN_DPAD_RIGHT);
static WIFI_ON:     EventCode = EventCode::EV_KEY(EV_KEY::BTN_TR);
static WIFI_OFF:    EventCode = EventCode::EV_KEY(EV_KEY::BTN_TR2);
static SUSPEND:     EventCode = EventCode::EV_KEY(EV_KEY::BTN_NORTH);

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


fn process_event(_dev: &Device, ev: &InputEvent, hotkey: bool) {
    /*println!("Event: time {}.{} type {} code {} value {} hotkey {}",
             ev.time.tv_sec,
             ev.time.tv_usec,
             ev.event_type,
             ev.event_code,
             ev.value,
             hotkey);*/

    if hotkey && ev.value == 1 {
        if ev.event_code == MUTE {
            Command::new("amixer").args(&["sset", "Playback", "0"]).output().expect("Failed to execute amixer");
        }
        else if ev.event_code == VOL_NORM {
            Command::new("amixer").args(&["sset", "Playback", "180"]).output().expect("Failed to execute amixer");
        }
        else if ev.event_code == PERF_MAX {
            Command::new("perfmax").arg("none").output().expect("Failed to execute performance");
            blinkon();
        }
        else if ev.event_code == PERF_NORM {
            Command::new("perfnorm").arg("none").output().expect("Failed to execute performance");
            blinkoff();
        }
        else if ev.event_code == DARK_ON {
            Command::new("brightnessctl").args(&["s","10%"]).output().expect("Failed to execute brightnessctl");
        }
        else if ev.event_code == DARK_OFF {
            Command::new("brightnessctl").args(&["s","50%"]).output().expect("Failed to execute brightnessctl");
        }
        else if ev.event_code == WIFI_ON {
            blinkon();
            Command::new("nmcli").args(&["radio","wifi","on"]).output().expect("Failed to execute wifi on");
        }
        else if ev.event_code == WIFI_OFF {
            Command::new("nmcli").args(&["radio","wifi","off"]).output().expect("Failed to execute wifi off");
            blinkoff();
        }
        else if ev.event_code == SUSPEND {
            Command::new("sudo").args(&["systemctl", "suspend"]).output().expect("Failed to execute power off");
        }
    }
    else if ev.event_code == BRIGHT_DOWN && ev.value == 1 {
        Command::new("brightnessctl").args(&["-n","s","2%-"]).output().expect("Failed to execute brightnessctl");
    }
    else if ev.event_code == BRIGHT_UP && ev.value == 1 {
        Command::new("brightnessctl").args(&["s","+2%"]).output().expect("Failed to execute brightnessctl");
    }
    else if ev.event_code == VOL_UP && ev.value == 1 {
        Command::new("amixer").args(&["-q", "sset", "Playback", "1%+"]).output().expect("Failed to execute amixer");
    }
    else if ev.event_code == VOL_DOWN && ev.value == 1 {
        Command::new("amixer").args(&["-q", "sset", "Playback", "1%-"]).output().expect("Failed to execute amixer");
    } 
}

fn main() -> io::Result<()> {
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1);
    let mut devs: Vec<Device> = Vec::new();
    let mut hotkey = false;

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
                        if ev.event_code == HOTKEY {
                            hotkey = ev.value == 1;
                            //let grab = if hotkey { GrabMode::Grab } else { GrabMode::Ungrab };
                            //dev.grab(grab)?;
                        }
                        process_event(&dev, &ev, hotkey)
                    },
                    _ => ()
                }
            }
        }
    }
}
