## Odroid Go Super
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


Prequisites
===========
```
sudo apt install brightnessctl rustc autotools-dev automake libtool libtool-bin libevdev-dev
```

Build
=====
```
git clone https://github.com/southoz/ogage.git
cd ogage
cargo build --release
strip target/release/ogage
```
ogage executable will be in the target/release folder.
