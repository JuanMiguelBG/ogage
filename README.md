## Odroid Go Super
![](https://github.com/southoz/RetroOZ/blob/main/wiki/images/Buttons_OGS.png)
### Hotkey - 17
* 0 (B Button) - Volume Up
* 1 (A Button) - Volume 75%
* 2 (X Button) - Volume Down
* 1 (Y Button) - Volume Mute
* 4 (L1 Button) - Performance Normal
* 6 (L2 Button) - Performance Max
* 5 (R2 Button) - Wifi On
* 7 (R2 Button) - Wifi Off
* 8 (D-Pad Up) - Brightness Up
* 9 (D-Pad Down) - Brightness Down
* 10 (D-Pad Left) - Brightness 10%
* 11 (D-Pad Right) - Brightness 50%
* 13 (Start) - Sleep

-----

## Odroid Go Advance
![](https://github.com/southoz/RetroOZ/blob/main/wiki/images/Buttons_OGA.png)
* 13 - Volume Down
* 14 - Volume Up
* 15 - Brightness Down
* 16 - Brightness Up
### Hotkey - 17
* 2 (XButton) - Sleep
* 4 (L1 Button) - Performance Normal
* 6 (L2 Button) - Performance Max
* 5 (R2 Button) - Wifi On
* 7 (R2 Button) - Wifi Off
* 8 (D-Pad Up) - Volume 75%
* 9 (D-Pad Down) - Volume Mute
* 10 (D-Pad Left) - Brightness 10%
* 11 (D-Pad Right) - Brightness 50%

-----

## RGB10 Max - Top
![](https://github.com/southoz/RetroOZ/blob/main/wiki/images/Buttons_RGB10_Max.png)
### Hotkey - 15
* 0 (B Button) - Volume Up
* 1 (A Button) - Volume 75%
* 2 (X Button) - Volume Down
* 1 (Y Button) - Volume Mute
* 4 (L1 Button) - Performance Normal
* 6 (L2 Button) - Performance Max
* 5 (R2 Button) - Wifi On
* 7 (R2 Button) - Wifi Off
* 8 (D-Pad Up) - Brightness Up
* 9 (D-Pad Down) - Brightness Down
* 10 (D-Pad Left) - Brightness 10%
* 11 (D-Pad Right) - Brightness 50%
* 13 (Start) - Sleep

-----

## RGB10 Max - Native
![](https://github.com/southoz/RetroOZ/blob/main/wiki/images/Buttons_RGB10_Max.png)
### Hotkey - 13
* 0 (B Button) - Volume Up
* 1 (A Button) - Volume 75%
* 2 (X Button) - Volume Down
* 1 (Y Button) - Volume Mute
* 4 (L1 Button) - Performance Normal
* 6 (L2 Button) - Performance Max
* 5 (R2 Button) - Wifi On
* 7 (R2 Button) - Wifi Off
* 8 (D-Pad Up) - Brightness Up
* 9 (D-Pad Down) - Brightness Down
* 10 (D-Pad Left) - Brightness 10%
* 11 (D-Pad Right) - Brightness 50%
* 15 (Start) - Sleep

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
git clone https://github.com/JuanMiguelBG/ogage.git
cd ogage
cargo build --release
strip target/release/ogage
```

ogage executable will be in the target/release folder.
