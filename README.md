## RG503
![](./rg503.png)

Global Hotkey: Right Thumbstick (R3)

R3 and Dpad Up = Brightness Up
R3 and Dpad Down = Brightness Down
R3 and Dpad Left = Dark on
R3 and Dpad Right = Dark off
R3 and Volume Up = Brightness Up (Can be held for continuous brightness increase)
R3 and Volume Down = Brightness Down (Can be held for continuous brightness decrease)
R3 and X = Volume Up
R3 and B = Volume Down
R3 and Y = Mute
R3 and A = Volume 75%
R3 and L1 = Normal Performance
R3 and L2 = Max Performance
R3 and R1 = Wifi On
R3 and R2 = Wifi Off
R3 and Power = Safely shutdown device
Power (Short Press) = Put device to sleep

-----


Prequisites
===========
You need at least Rust version 1.5.1. If you use Christians pre built virtual machine image with a chroot for arm64 https://forum.odroid.com/viewtopic.php?p=306185#p306185 use

```
apt install brightnessctl autotools-dev automake libtool libtool-bin libevdev-dev
```

and download and install

```
https://static.rust-lang.org/rustup/dist/aarch64-unknown-linux-gnu/rustup-init
```

Select platform "aarch64-unknown-linux-gnu", version "stable" and "minimal".


To compile from device:

```
sudo apt install brightnessctl rustc autotools-dev automake libtool libtool-bin libevdev-dev
```

Build
=====
```
git clone https://github.com/JuanMiguelBG/ogage.git -b 503
cd ogage
cargo build --release
strip target/release/ogage
```

ogage executable will be in the target/release folder.
