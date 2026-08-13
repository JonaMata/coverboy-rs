# CoverBoy
CoverBoy shows the album cover of the current playing track on a 64*64 LED matrix connected to a Raspberry Pi.

To avoid having to configure many APIs, CoverBoy uses the Home Assistant Websocket API to get the current playing track and the cover image.
CoverBoy should work with any music player that is integrated with Home Assistant, this includes Spotify, Sonos, Heos, and many others.

## Hardware
CoverBoy should work with any combination of 64-bit Raspberry Pi, led-matrix hat and 64*64 LED matrix that is compatible with the [rpi-rgb-led-matrix](https://github.com/hzeller/rpi-rgb-led-matrix) library.
For good performance, the sound module of the Raspberry Pi should be disabled by adding `dtparam=audio=off` to your /boot/config.txt.

## Installation
1. Download the latest `.deb` release from the [releases page](https://github.com/JonaMata/CoverBoy-C/releases).
2. Install the `.deb` package with `sudo dpkg -i coverboy_x.y.z_arm64.deb` (`x.y.z` being the version of the downloaded `.deb` file).
3. During installation, you will be asked for the URL of your Home Assistant instance and an access token.
   The access token can be generated in your Home Assistant profile settings under the security tab.
4. After installation, the service `coverboy` will be started automatically.

## Configuration
CoverBoy will look for the `/etc/coverboy.conf` configuration file.

The configuration file is a simple key-value pair file and looks like this:
```sh
# Configuration file for CoverBoy
HA_URL="https://homeassistant.local:8123"
HA_TOKEN="long_lived_access_token"
```

## Usage
If you used the `.deb` package, the service `coverboy` will be started and enabled automatically.
You can start, stop, and restart the service with `sudo systemctl start coverboy`, `sudo systemctl stop coverboy`, and `sudo systemctl restart coverboy` respectively.
If you don't want CoverBoy to start automatically, you can disable the service with `sudo systemctl disable coverboy`.

If you only downloaded the executable, you can start CoverBoy with `./coverboy`. Make sure to have a correct `coverboy.conf` in the same directory or in `/etc/coverboy.conf`.

## Building from source
CoverBoy is built in Rust and uses Cargo.
To compile for the Raspberry Pi you need to have a working cross-compilation toolchain,
and have it correctly configured in .cargo/config.toml.
The executable or .deb package can then be built by running `cargo [build|deb] --target aarch64-unknown-linux-gnu`