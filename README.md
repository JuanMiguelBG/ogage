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

Prequisites
===========
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
