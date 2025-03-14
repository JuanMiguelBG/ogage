#[macro_use]
extern crate lazy_static;
extern crate evdev_rs as evdev;
extern crate mio;

use evdev::enums::*;
use evdev::*;
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};
use props_rs::*;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime};

static PERF_MAX: EventCode = EventCode::EV_KEY(EV_KEY::BTN_TL2);
static PERF_NORM: EventCode = EventCode::EV_KEY(EV_KEY::BTN_TL);
static DARK_ON: EventCode = EventCode::EV_KEY(EV_KEY::BTN_DPAD_LEFT);
static DARK_OFF: EventCode = EventCode::EV_KEY(EV_KEY::BTN_DPAD_RIGHT);
static WIFI_ON: EventCode = EventCode::EV_KEY(EV_KEY::BTN_TR);
static WIFI_OFF: EventCode = EventCode::EV_KEY(EV_KEY::BTN_TR2);
static POWER_OFF: EventCode = EventCode::EV_KEY(EV_KEY::KEY_POWER);
static MIN_POWERKEY_ELAPSED: Duration = Duration::from_secs(1);
static DEVICE_FILE: &'static str = "/opt/.retrooz/device";
static POWERKEY_CFG_FILE: &'static str = "/usr/local/etc/powerkey.conf";
static OGAGE_CFG_FILE: &'static str = "/usr/local/etc/ogage.conf";
static AUTO_SUSPEND_CFG_FILE: &'static str = "/usr/local/etc/auto_suspend.conf";
static AUTO_DIM_CFG_FILE: &'static str = "/usr/local/etc/auto_dim.conf";
static BATTERY_STATUS_FILE: &'static str = "/sys/class/power_supply/battery/status";

enum PowerkeyActions {
    Shutdown,
    Suspend,
    Disabled
}

enum BatteryStatus {
    Unknown,
    Charging,
    Discharging,
    NotCharging,
    Full,
}

lazy_static! {
    static ref DEVICE: &'static str = {
        if Path::new(DEVICE_FILE).exists() {
            let lines = fs::read_to_string(DEVICE_FILE).expect(&("Can't read file '".to_owned() + DEVICE_FILE + "'.")).trim_end_matches(&['\r', '\n'][..]).to_string();
            if !lines.is_empty() {
                return Box::leak(lines.into_boxed_str());
            }
        }

        "rgb10maxtop"
    };

    static ref IS_OGA1: bool = {
        if *DEVICE == "oga1" {
            return true;
        }
        // OGS, OGA, RGB10 MAX/MAX2
        false
    };

    static ref HOTKEY: EventCode = {
        let device_str = DEVICE.to_string();
        if device_str.starts_with("rgb10max") {
            if device_str.ends_with("top") {
                return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY4);
            }
            else { // native
                return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY2);
            }
        }

        // OGS, OGA and OGA 1.1
        EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY6)
    };

    static ref BRIGHT_UP: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY5);
        }

        // OGS, OGA, RGB10 MAX/MAX2
        EventCode::EV_KEY(EV_KEY::BTN_DPAD_UP)
    };

    static ref BRIGHT_DOWN: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY4);
        }

        // OGS, OGA, RGB10 MAX/MAX2
        EventCode::EV_KEY(EV_KEY::BTN_DPAD_DOWN)
    };

    static ref VOL_UP: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY3);
        }

        // OGS, OGA, RGB10 MAX/MAX2
        EventCode::EV_KEY(EV_KEY::BTN_NORTH)
    };

    static ref VOL_DOWN: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY2);
        }

        // OGS, OGA, RGB10 MAX/MAX2
        EventCode::EV_KEY(EV_KEY::BTN_SOUTH)
    };

    static ref MUTE: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_DPAD_DOWN);
        }

        // OGS, OGA, RGB10 MAX/MAX2
        EventCode::EV_KEY(EV_KEY::BTN_WEST)
    };

    static ref VOL_NORM: EventCode = {
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_DPAD_UP);
        }

        // OGS, OGA, RGB10 MAX/MAX2
        EventCode::EV_KEY(EV_KEY::BTN_EAST)
    };

    static ref SUSPEND: EventCode = {
        let device_str = DEVICE.to_string();
        if *IS_OGA1 {
            return EventCode::EV_KEY(EV_KEY::BTN_NORTH);
        }
        else if device_str.starts_with("rgb10max") && device_str.ends_with("native") {
            return EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY4);
        }

        // OGS, OGA, RGB10 MAX/MAX2 Top
        EventCode::EV_KEY(EV_KEY::BTN_TRIGGER_HAPPY2)
    };

    static ref POWERKEY_PROPERTIES: HashMap<String, String> = {
        println!("\nPOWERKEY_PROPERTIES:");
        if Path::new(POWERKEY_CFG_FILE).exists() {
            let lines = fs::read_to_string(POWERKEY_CFG_FILE).expect(&("Can't read file '".to_owned() + POWERKEY_CFG_FILE + "'."));
            let parsed = parse(lines.as_bytes()).expect(&("Can't parse properties of '".to_owned() + POWERKEY_CFG_FILE + "'."));

            let map_properties = to_map(parsed);

            for (key, value) in map_properties.iter() {
                println!("\t{} / {}", key, value);
            }
            println!("\n");
            return map_properties;
        }

        HashMap::new()
    };

    static ref IS_DOUBLE_PUSH_POWERKEY_ACTIVE: bool = {
        if !POWERKEY_PROPERTIES.is_empty() {
            match POWERKEY_PROPERTIES.get("two_push_shutdown") {
                Some(x) => {
                    if x == "enabled" {
                        return true;
                    }
                },
                _ => ()
            };
        }

        false
    };

    static ref MAX_POWERKEY_INTERVAL_TIME: Duration = {
        if !POWERKEY_PROPERTIES.is_empty() {
            match POWERKEY_PROPERTIES.get("max_interval_time") {
                Some(x) => return Duration::from_secs(x.parse::<u64>().unwrap() + MIN_POWERKEY_ELAPSED.as_secs()),
                None => return Duration::from_secs(2),
            };
        }

        Duration::from_secs(2)
    };

    static ref POWERKEY_ACTION: PowerkeyActions = {
        if !POWERKEY_PROPERTIES.is_empty() {
            match POWERKEY_PROPERTIES.get("action") {
                Some(x) => {
                    if x == "suspend" {
                        return PowerkeyActions::Suspend;
                    }
                    else if x == "disabled" {
                        return PowerkeyActions::Disabled;
                    }
                },
                _ => ()
            };
        }

        PowerkeyActions::Shutdown
    };

    static ref AUTO_SUSPEND_PROPERTIES: HashMap<String, String> = {
        println!("\nAUTO_SUSPEND_PROPERTIES:");
        if Path::new(AUTO_SUSPEND_CFG_FILE).exists() {
            let lines = fs::read_to_string(AUTO_SUSPEND_CFG_FILE).expect(&("Can't read file '".to_owned() + AUTO_SUSPEND_CFG_FILE + "'."));
            let parsed = parse(lines.as_bytes()).expect(&("Can't parse properties of '".to_owned() + AUTO_SUSPEND_CFG_FILE + "'."));

            let map_properties = to_map(parsed);

            for (key, value) in map_properties.iter() {
                println!("\t{} / {}", key, value);
            }
            println!("\n");
            return map_properties;
        }

        HashMap::new()
    };

    static ref AUTO_SUSPEND_ENABLED: bool = {
        if !AUTO_SUSPEND_PROPERTIES.is_empty() {
            match AUTO_SUSPEND_PROPERTIES.get("auto_suspend_time") {
                Some(x) => {
                    if x == "enabled" {
                        return true;
                    }
                },
                _ => ()
            };
        }

        false
    };

    // timeout in minutes
    static ref AUTO_SUSPEND_TIMEOUT: Duration = {
        if !AUTO_SUSPEND_PROPERTIES.is_empty() {
            match AUTO_SUSPEND_PROPERTIES.get("auto_suspend_timeout") {
                Some(x) => return Duration::from_secs(x.parse::<u64>().unwrap() * 60),
                _ => ()
            };
        }

        Duration::from_secs(300)
    };

    static ref AUTO_SUSPEND_STAY_AWAKE_WHILE_CHARGING: bool = {
        if !AUTO_SUSPEND_PROPERTIES.is_empty() {
            match AUTO_SUSPEND_PROPERTIES.get("auto_suspend_stay_awake_while_charging") {
                Some(x) => {
                    if x == "enabled" {
                        return true;
                    }
                },
                _ => ()
            };
        }

        false
    };

    static ref AUTO_DIM_PROPERTIES: HashMap<String, String> = {
        println!("\nAUTO_DIM_PROPERTIES:");
        if Path::new(AUTO_DIM_CFG_FILE).exists() {
            let lines = fs::read_to_string(AUTO_DIM_CFG_FILE).expect(&("Can't read file '".to_owned() + AUTO_DIM_CFG_FILE + "'."));
            let parsed = parse(lines.as_bytes()).expect(&("Can't parse properties of '".to_owned() + AUTO_DIM_CFG_FILE + "'."));

            let map_properties = to_map(parsed);

            for (key, value) in map_properties.iter() {
                println!("\t{} / {}", key, value);
            }
            println!("\n");
            return map_properties;
        }

        HashMap::new()
    };
    
    static ref AUTO_DIM_STAY_AWAKE_WHILE_CHARGING: bool = {
        if !AUTO_DIM_PROPERTIES.is_empty() {
            match AUTO_DIM_PROPERTIES.get("auto_dim_stay_awake_while_charging") {
                Some(x) => {
                    if x == "enabled" {
                        return true;
                    }
                },
                _ => ()
            };
        }

        false
    };

    static ref AUTO_DIM_ENABLED: bool = {
        if !AUTO_DIM_PROPERTIES.is_empty() {
            match AUTO_DIM_PROPERTIES.get("auto_dim_time") {
                Some(x) => {
                    if x == "enabled" {
                        return true;
                    }
                },
                _ => ()
            };
        }

        false
    };

    // timeout in minutes
    static ref AUTO_DIM_TIMEOUT: Duration = {
        if !AUTO_DIM_PROPERTIES.is_empty() {
            match AUTO_DIM_PROPERTIES.get("auto_dim_timeout") {
                Some(x) => return Duration::from_secs(x.parse::<u64>().unwrap() * 60),
                _ => ()
            };
        }

        Duration::from_secs(300)
    };

    // brightness level
    static ref AUTO_DIM_BRIGHTNESS: u32 = {
        if !AUTO_DIM_PROPERTIES.is_empty() {
            match AUTO_DIM_PROPERTIES.get("auto_dim_brightness") {
                Some(x) => return x.parse::<u32>().unwrap(),
                _ => ()
            };
        }

        10
    };

    static ref OGAGE_PROPERTIES: HashMap<String, String> = {
        println!("\nOGAGE PROPERTIES:");
        if Path::new(OGAGE_CFG_FILE).exists() {
            let lines = fs::read_to_string(OGAGE_CFG_FILE).expect(&("Can't read file '".to_owned() + OGAGE_CFG_FILE + "'."));
            let parsed = parse(lines.as_bytes()).expect(&("Can't parse properties of '".to_owned() + OGAGE_CFG_FILE + "'."));

            let map_properties = to_map(parsed);

            for (key, value) in map_properties.iter() {
                println!("\t{} / {}", key, value);
            }
            println!("\n");
            return map_properties;
        }

        HashMap::new()
    };

    static ref ALLOW_BRIGHTNESS: bool = {
        if !OGAGE_PROPERTIES.is_empty() {
            match OGAGE_PROPERTIES.get("brightness") {
                Some(x) => {
                    if x == "disabled" {
                        return false;
                    }
                },
                _ => ()
            };
        }

        true
    };

    static ref BRIGHTNESS_STEP: u32 = {
        if !OGAGE_PROPERTIES.is_empty() {
            match OGAGE_PROPERTIES.get("brightness_step") {
                Some(x) => return x.parse::<u32>().unwrap(),
                _ => ()
            };
        }

        1
    };

    static ref ALLOW_VOLUME: bool = {
        if !OGAGE_PROPERTIES.is_empty() {
            match OGAGE_PROPERTIES.get("volume") {
                Some(x) => {
                    if x == "disabled" {
                        return false;
                    }
                },
                _ => ()
            };
        }

        true
    };

    static ref VOLUME_STEP: u32 = {
        if !OGAGE_PROPERTIES.is_empty() {
            match OGAGE_PROPERTIES.get("volume_step") {
                Some(x) => return x.parse::<u32>().unwrap(),
                _ => ()
            };
        }

        1
    };

    static ref ALLOW_WIFI: bool = {
        if !OGAGE_PROPERTIES.is_empty() {
            match OGAGE_PROPERTIES.get("wifi") {
                Some(x) => {
                    if x == "disabled" {
                        return false;
                    }
                },
                _ => ()
            };
        }

        true
    };

    static ref ALLOW_PERFORMANCE: bool = {
        if !OGAGE_PROPERTIES.is_empty() {
            match OGAGE_PROPERTIES.get("performance") {
                Some(x) => {
                    if x == "disabled" {
                        return false;
                    }
                },
                _ => ()
            };
        }

        true
    };

    static ref ALLOW_SUSPEND: bool = {
        if !OGAGE_PROPERTIES.is_empty() {
            match OGAGE_PROPERTIES.get("suspend") {
                Some(x) => {
                    if x == "disabled" {
                        return false;
                    }
                },
                _ => ()
            };
        }

        true
    };

    static ref ES_BRIGTHNESS_LOCK_FILE: &'static str = {
        if !OGAGE_PROPERTIES.is_empty() {
            match OGAGE_PROPERTIES.get("es_brigthness_lock_file") {
                Some(x) => {
                    return x;
                },
                _ => ()
            };
        }

        "/home/odroid/.emulationstation/brightness.lock"
    };
}

fn get_brightness() -> u32 {
    let output = Command::new("brightnessctl")
        .arg("-m")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute brightnessctl");
    let brightness_str =
        String::from_utf8(output.stdout).expect("Failed to convert stdout to string");

    let brightness_vector: Vec<&str> = brightness_str.split(&[',', '%'][..]).collect();

    if brightness_vector.len() > 3 {
        //println!("Get volume level: {}%", brightness_vector[3]);
        return brightness_vector[3]
                .trim()
                .parse::<u32>()
                .expect("Failed to parse brightness string")
    }

    50
}

fn set_brightness(brightness: u32) {
    let brightness_str = brightness.to_string() + "%";
    //println!("Set brightness level: {}", brightness_str);
    Command::new("brightnessctl")
        .args(&["s", &brightness_str])
        .output()
        .expect("Failed to execute brightnessctl");
}

fn blinkon() {
    let current = get_brightness();
    create_es_brightness_lock();
    set_brightness(0);
    thread::sleep(Duration::from_millis(200));
    create_es_brightness_lock();
    set_brightness(100);
    thread::sleep(Duration::from_millis(200));
    create_es_brightness_lock();
    set_brightness(0);
    thread::sleep(Duration::from_millis(200));
    create_es_brightness_lock();
    set_brightness(current);
    remove_es_brightness_lock();
}

fn blinkoff() {
    let current = get_brightness();
    create_es_brightness_lock();
    set_brightness(0);
    thread::sleep(Duration::from_millis(300));
    create_es_brightness_lock();
    set_brightness(current);
    remove_es_brightness_lock();
}

fn get_volume() -> u32 {
    let output = Command::new("amixer")
        .args(&["sget", "Playback"])
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute amixer");

    let lines_str =
        String::from_utf8(output.stdout).expect("Failed to convert stdout to string");

    let last_line = lines_str
        .lines()
        .last()
        .expect("could not get volume info");

    let last = last_line
        .split_whitespace()
        .filter(|x| x.starts_with('[') && !x.contains("dB"))
        .map(|s| s.trim_matches(&['[', ']', '%'] as &[_]))
        .collect::<Vec<&str>>();

    if last.len() > 0 {
        //println!("Get volume level: {}%", last[0]);
        return last[0]
                .trim()
                .parse::<u32>()
                .expect("Failed to parse volume  string")
    }

    50
}

fn set_volume(volume: u32) {
    let volume_str = volume.to_string() + "%";
    //println!("Set volume level: {}", volume_str);
    Command::new("amixer")
        .args(&["sset", "Playback", &volume_str])
        .output()
        .expect("Failed to execute amixer");
}

fn inc_brightness() {
    let mut brightness = get_brightness();
    if brightness < *BRIGHTNESS_STEP {
        brightness = *BRIGHTNESS_STEP;
    }
    else {
        brightness = brightness + *BRIGHTNESS_STEP;
    }
    set_brightness(brightness);
}

fn dec_brightness() {
    let mut brightness = get_brightness();
    if brightness <= *BRIGHTNESS_STEP {
        brightness = 1;
    }
    else {
        brightness = brightness - *BRIGHTNESS_STEP;
    }
    set_brightness(brightness);
}

fn inc_volume() {
    let mut volume = get_volume();
    if volume < *VOLUME_STEP {
        volume = *VOLUME_STEP;
    }
    else {
        volume = volume + *VOLUME_STEP;
    }
    set_volume(volume);
}

fn dec_volume() {
    let mut volume = get_volume();
    if volume < *VOLUME_STEP {
        volume = 0;
    }
    else {
        volume = volume - *VOLUME_STEP;
    }
    set_volume(volume);
}

fn mute_volume() {
    set_volume(0);
}

fn norm_volume() {
    set_volume(75);
}

fn perf_max() {
    Command::new("perfmax")
        .arg("none")
        .output()
        .expect("Failed to execute performance");
    blinkon();
}

fn perf_norm() {
    Command::new("perfnorm")
        .arg("none")
        .output()
        .expect("Failed to execute performance");
    blinkoff();
}

fn dark_on() {
    set_brightness(10);
}

fn dark_off() {
    set_brightness(50);
}

fn wifi_on() {
    blinkon();
    Command::new("nmcli")
        .args(&["radio", "wifi", "on"])
        .output()
        .expect("Failed to execute wifi on");
}

fn wifi_off() {
    Command::new("nmcli")
        .args(&["radio", "wifi", "off"])
        .output()
        .expect("Failed to execute wifi off");
    blinkoff();
}

fn suspend() {
    Command::new("sudo")
        .args(&["systemctl", "suspend"])
        .output()
        .expect("Failed to execute suspend");
}

fn power_off() {
    Command::new("sudo")
        .args(&["shutdown", "-h", "now"])
        .output()
        .expect("Failed to execute power off");
}

fn battery_status() -> BatteryStatus {
    let status_str =
        fs::read_to_string(BATTERY_STATUS_FILE).expect("Failed to read battery status");

    match status_str.as_str().trim() {
        "Unknown" => BatteryStatus::Unknown,
        "Charging" => BatteryStatus::Charging,
        "Discharging" => BatteryStatus::Discharging,
        "Not charging" => BatteryStatus::NotCharging,
        "Full" => BatteryStatus::Full,
        _ => panic!("Unhandled battery status value"),
    }
}

fn create_es_brightness_lock() {
    if !Path::new(*ES_BRIGTHNESS_LOCK_FILE).exists() {
        std::fs::File::create(*ES_BRIGTHNESS_LOCK_FILE).expect("create failed");
    }
}

fn remove_es_brightness_lock() {
    if Path::new(*ES_BRIGTHNESS_LOCK_FILE).exists() {
        thread::sleep(Duration::from_millis(200));
        fs::remove_file(*ES_BRIGTHNESS_LOCK_FILE).expect("remove failed");
    }    
}

fn process_oga1_event(ev: &InputEvent) {
    if ev.event_code == *BRIGHT_UP && *ALLOW_BRIGHTNESS {
        inc_brightness();
    } else if ev.event_code == *BRIGHT_DOWN && *ALLOW_BRIGHTNESS {
        dec_brightness();
    } else if ev.event_code == *VOL_UP && *ALLOW_VOLUME {
        inc_volume();
    } else if ev.event_code == *VOL_DOWN && *ALLOW_VOLUME {
        dec_volume();
    }
}

fn process_event(_dev: &Device, ev: &InputEvent, hotkey: bool) {
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
        println!("IS double push power off button active?: {}", *IS_DOUBLE_PUSH_POWERKEY_ACTIVE);
        */

        if hotkey {
            if !*IS_OGA1 {
                process_oga1_event(ev);
            }
            if ev.event_code == *MUTE && *ALLOW_VOLUME {
                mute_volume();
            } else if ev.event_code == *VOL_NORM && *ALLOW_VOLUME {
                norm_volume();
            } else if ev.event_code == PERF_MAX && *ALLOW_PERFORMANCE {
                perf_max();
            } else if ev.event_code == PERF_NORM && *ALLOW_PERFORMANCE {
                perf_norm();
            } else if ev.event_code == DARK_ON && *ALLOW_BRIGHTNESS {
                dark_on();
            } else if ev.event_code == DARK_OFF && *ALLOW_BRIGHTNESS {
                dark_off();
            } else if ev.event_code == WIFI_ON && *ALLOW_WIFI {
                wifi_on();
            } else if ev.event_code == WIFI_OFF && *ALLOW_WIFI {
                wifi_off();
            } else if ev.event_code == *SUSPEND && *ALLOW_SUSPEND {
                suspend();
            }
        } else if *IS_OGA1 {
            process_oga1_event(ev);
        }
    }
}

fn main() -> io::Result<()> {
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1);
    let mut devs: Vec<Device> = Vec::new();
    let mut hotkey = false;
    let mut first_push_powerkey: Option<SystemTime> = None;
    let mut last_button_push: SystemTime = SystemTime::now();
    let mut last_charge: SystemTime = SystemTime::now();
    let mut auto_dim_active: bool = false;
    let mut last_brightness: u32 = 0;

    println!("\nDevice: {}\nIs OGA v1.1?: {}\nIs double push power off button active?: {}\nPOWERKEY interval time: {:?}\nPOWERKEY action: {}\nAuto suspend: {}\nAuto suspend timeout: {:?}\nAuto suspend stay awake while charging: {}\nAuto dim: {}\nAuto dim timeout: {:?}\nAuto dim brightness: {}%\nAuto dim stay awake while charging: {}",
             *DEVICE, *IS_OGA1, *IS_DOUBLE_PUSH_POWERKEY_ACTIVE, *MAX_POWERKEY_INTERVAL_TIME,
             match *POWERKEY_ACTION {
                PowerkeyActions::Suspend => "suspend",
                PowerkeyActions::Disabled => "disabled",
                _ => "shutdown",
            }, *AUTO_SUSPEND_ENABLED, *AUTO_SUSPEND_TIMEOUT, *AUTO_SUSPEND_STAY_AWAKE_WHILE_CHARGING, *AUTO_DIM_ENABLED, *AUTO_DIM_TIMEOUT, *AUTO_DIM_BRIGHTNESS, *AUTO_DIM_STAY_AWAKE_WHILE_CHARGING);

    println!("Allow brightness: {}\nBrightness step: {}%\nAllow volume: {}\nVolume step: {}%\nAllow wifi: {}\nAllow performance: {}\nAllow suspend: {}", 
        *ALLOW_BRIGHTNESS, *BRIGHTNESS_STEP, *ALLOW_VOLUME, *VOLUME_STEP, *ALLOW_WIFI, *ALLOW_PERFORMANCE, *ALLOW_SUSPEND);
   
    println!("Emulationstation Brighthness Lock File: {}", *ES_BRIGTHNESS_LOCK_FILE);

    let mut i = 0;
    for s in [
        "/dev/input/event3",
        "/dev/input/event2",
        "/dev/input/event0",
        "/dev/input/event1",
    ]
    .iter()
    {
        if !Path::new(s).exists() {
            println!("Path {} doesn't exist", s);
            continue;
        }
        let fd = File::open(Path::new(s)).unwrap();
        let mut dev = Device::new().unwrap();
        poll.registry()
            .register(&mut SourceFd(&fd.as_raw_fd()), Token(i), Interest::READABLE)?;
        dev.set_fd(fd)?;
        devs.push(dev);
        println!("Added device {}", s);
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

                        process_event(&dev, &ev, hotkey);

                        if ev.event_code == POWER_OFF 
                            && ev.value == 1
                        {
                            if *IS_DOUBLE_PUSH_POWERKEY_ACTIVE    
                            {
                                let next_first_push_powerkey: SystemTime = SystemTime::now();
                                if first_push_powerkey.is_some() {
                                    let diff = first_push_powerkey.unwrap().elapsed().unwrap();
                                    first_push_powerkey = Some(next_first_push_powerkey);
                                    //println!("diff: {:?})", diff);
                                    if diff >= MIN_POWERKEY_ELAPSED
                                        && diff <= *MAX_POWERKEY_INTERVAL_TIME
                                    {
                                        // two push at least in more than one second
                                        match *POWERKEY_ACTION {
                                            PowerkeyActions::Suspend => suspend(),
                                            PowerkeyActions::Shutdown => power_off(),
                                            _ => ()
                                        }
                                    }
                                } else {
                                    first_push_powerkey = Some(next_first_push_powerkey);
                                }
                            }
                            else {
                                match *POWERKEY_ACTION {
                                    PowerkeyActions::Suspend => suspend(),
                                    _ => ()
                                }
                            }

                        }

                        // Variables used for both auto-suspend and auto-dimming
                        let button_pushed = ev.value == 1;

                        let charging: bool = match battery_status() {
                            BatteryStatus::Unknown => false,
                            BatteryStatus::Charging => true,
                            BatteryStatus::Discharging => false,
                            BatteryStatus::NotCharging => false,
                            BatteryStatus::Full => true,
                        };

                        if button_pushed {
                            last_button_push = SystemTime::now();
                        }

                        if charging {
                            last_charge = SystemTime::now();
                        }

                        if *AUTO_SUSPEND_ENABLED {
                            /*
                            println!("Event: time {}.{} type {} code {} value {} hotkey {}\nLast Push Button Time {:?}\nActual Time {:?}\n",
                                     ev.time.tv_sec, ev.time.tv_usec, ev.event_type, ev.event_code,
                                     ev.value, hotkey, last_button_push, SystemTime::now());
                            */
                            let button_push_timed_out =
                                last_button_push.elapsed().unwrap() > *AUTO_SUSPEND_TIMEOUT;
                            let charge_timed_out =
                                last_charge.elapsed().unwrap() > *AUTO_SUSPEND_TIMEOUT;

                            if (*AUTO_SUSPEND_STAY_AWAKE_WHILE_CHARGING
                                && button_push_timed_out
                                && charge_timed_out)
                                || (!*AUTO_SUSPEND_STAY_AWAKE_WHILE_CHARGING
                                    && button_push_timed_out)
                            {
                                suspend();
                                last_button_push = SystemTime::now();
                                last_charge = SystemTime::now();
                            }
                        }

                        if *AUTO_DIM_ENABLED {
                            if auto_dim_active {
                                if button_pushed
                                    || (*AUTO_DIM_STAY_AWAKE_WHILE_CHARGING && charging)
                                {
                                    // Restore previous brightness
                                    auto_dim_active = false;
                                    create_es_brightness_lock();
                                    set_brightness(last_brightness);
                                    remove_es_brightness_lock();
                                }
                            } else {
                                let button_push_timed_out =
                                    last_button_push.elapsed().unwrap() > *AUTO_DIM_TIMEOUT;
                                let charge_timed_out =
                                    last_charge.elapsed().unwrap() > *AUTO_DIM_TIMEOUT;

                                if (*AUTO_DIM_STAY_AWAKE_WHILE_CHARGING
                                    && button_push_timed_out
                                    && charge_timed_out)
                                    || (!*AUTO_DIM_STAY_AWAKE_WHILE_CHARGING
                                        && button_push_timed_out)
                                {
                                    // Save current brightness and dim the screen
                                    auto_dim_active = true;
                                    last_brightness = get_brightness();
                                    create_es_brightness_lock();
                                    set_brightness(*AUTO_DIM_BRIGHTNESS);
                                    remove_es_brightness_lock();
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }
}
