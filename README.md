# What is this?

This is an utility for programming small keyboards like this one:

![](doc/keyboard-12-2.png)

Such keyboards are popular on AliExpress and seller usually sends software
for programming, but it:
 * requires Windows,
 * is very ugly and inconvenient,
 * can only program one key at a time
 * don't expose all keyboard features

There are several modifications of such keyboards with different number of
buttons and knobs ([see some photos](#photos)) and with/without Bluetooth.

Both wired and wireless keyboards are supported, however programming
is possible though wire only in both cases!

Utility was reported to work with:
 * 3×4 with 2 knobs (Bluetooth version)
 * 3×3 with 2 knobs
 * 3x2 with 1 knob
 * 3x1 with 1 knob

All these keyboards share same vendor/product IDs: 1189:8890 (hexadecimal).
It is possible to override used vendor/product ID, but it is usually not needed.
Use it only if you find same-looking keyboard with other vendor/product ID,
I haven't seen such.

**Ability to override vendor/product ID doesn't mean that you can use
this software for programming arbitrary keyboards!**

# How to get it?

## Get prebuilt release

Download latest release from [GitHub releases](https://github.com/kriomant/ch57x-keyboard-tool/releases)

## Build it yourself

Install *cargo* utility using [rustup](https://rustup.rs/), then execute
`cargo install ch57x-keyboard-tool`.

# How to use?

**Note**: on Windows you need to install [USBDK](https://github.com/daynix/UsbDk/releases) first.

Now create you own config from provided *example-mapping.yaml*, and apply:

    ch57x-keyboard-tool upload < your-config.yaml

You can also change LED configuration, if you keyboard supports it:

    ch57x-keyboard-tool led 1

# Diagnostics

If you have any troubles using this software, please provide diagnostics.

## Getting list of attached USB devices

### MacOS


    ioreg -w0 -l -p IOUSB

or

    system_profiler SPUSBDataType

### Linux


    lsusb -v

## Monitoring generated keyboard and mouse events

Most simple (and cross-platform) way I've found is using `keyboard` and `mouse` Python modules.

Monitoring keyboard:

    pip3 install keyboard
    sudo python3 -m keyboard

Monitoring mouse:

    # Latest published 'mouse' module doesn't support MacOS, so use latest version from Git:
    git clone https://github.com/boppreh/mouse
    cd mouse
    python3 -m mouse

## Photos

![](doc/keyboard-6-1.png)
![](doc/keyboard-3-1.jpg)
