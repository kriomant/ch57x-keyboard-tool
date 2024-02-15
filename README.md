# ch57x-keyboard-tool Macro Keyboard Configuration Utility

![Last Commit Shields.io](https://img.shields.io/github/last-commit/kriomant/ch57x-keyboard-tool?style=for-the-badge) ![Release Workflow Badge](https://github.com/kriomant/ch57x-keyboard-tool/actions/workflows/release.yml/badge.svg)

## What is this?

This is a utility for programming small keyboards like this one:

![Picture of keyboard-12-2](doc/keyboard-12-2.png)

Such macro keyboards are popular on AliExpress and sellers usually send software for programming, but it:
* requires Windows
* is very ugly and inconvenient
* can only program one key at a time
* don't expose all keyboard features

There are several modifications of such keyboards with different numbers of buttons and knobs (See the [photos of supported keyboards](#photos-of-supported-keyboards)) and with/without Bluetooth.

Both wired and wireless keyboards are supported.  
⚠️ However, the keyboard must be connected to the computer with the USB cable when programming it.

### Supported Keyboards

This utility was reported to work with:
* 3×4 with 2 knobs (Bluetooth version)
* 3×3 with 2 knobs
* 3x2 with 1 knob
* 3x1 with 1 knob (but [read about it's limitations](#3x1-keys--1-knob-keyboard-limitations))

All these keyboards share the same vendor/product IDs: `1189:8890` (hexadecimal).
It is possible to override used vendor/product ID, but it is usually not needed.
Use it only if you find same-looking keyboard with other vendor/product ID,
I haven't seen such.

Refer to the [Supported Macro Keyboards](#supported-macro-keyboards) section for more details.

**⚠️ Ability to override vendor/product ID doesn't mean that you can use this software for programming arbitrary keyboards!**

## Installation

There are two ways to get this software: prebuilt release or build it yourself.

### Get prebuilt release

Download the latest release from [GitHub releases](https://github.com/kriomant/ch57x-keyboard-tool/releases)

### Build it yourself

1. Install the *cargo* utility using [rustup](https://rustup.rs/)
    * Brew: `brew install rustup-init && rustup-init`
    * Linux: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
    * Windows: download and run [rustup-init.exe](https://win.rustup.rs/)
1. Execute `cargo install ch57x-keyboard-tool`.

## How to use?

**Note**: on Windows you need to install [USBDK](https://github.com/daynix/UsbDk/releases) first.

Now create you own config from provided *example-mapping.yaml*. Example
config has extensive documentation and examples inside.

You can validate config:

```shell
./ch57x-keyboard-tool validate < your-config.yaml
```

Use 'show-keys' command to list all supported modifier and key names.

Finally, upload config to keyboard:

```shell
./ch57x-keyboard-tool upload < your-config.yaml
```

Use 'sudo' if you get 'Access denied (insufficient permissions)':

    sudo ./ch57x-keyboard-tool upload < your-config.yaml

You can also change LED configuration, if you keyboard supports it:

```shell
./ch57x-keyboard-tool led 1
```

### Windows / PowerShell

Use `Get-Content` for input redireciton:

    Get-Content your-config.yaml | ./ch57x-keyboard-tool validate

### Automation

The frequent question is "How to run script / emulate several keys / … on key press?"
This tool does just one job — writes your key bindings into keyboard and then exists, it does not
listen for pressed key. If you want any automation, use third-party automation tools, like BetterTouchTool
or dozens of other.

1. Choose some chord you don't usually use, like 'alt-ctrl-shift-1' and assign to some key
2. Use third-party tool to listen for this chord and perform action you want.
3. Done!

## Notes

### Number of layers

All keyboards I've seen have three layer (three keys configuration which
may be switched). However I've been told there are keyboards without
layer switch. If so, just keep single layer in configuration file and you
are done.

### Custom keyboard layouts

If you use custom keyboard layout, like Dvorak, note that what you
write in configuration is in fact scan code of keyboard key and not
character that will be produced.

So use QWERTY-letter of keyboard key you want to press.

### 3x1 keys + 1 knob keyboard limitations

This modification does support key modifiers (like ctrl-, alt-) for the first key in sequence only.

So you can use: `ctrl-alt-del,1,2`, but not `ctrl-alt-del,alt-1,2`.

## Diagnostics

When reporting an issue, please include diagnostics such as the list of attached USB devices and the output of the `keyboard` and `mouse` monitoring tools.

### How to Find and List Connected USB Devices

#### macOS

```shell
system_profiler SPUSBDataType
```

or

```shell
ioreg -w0 -l -p IOUSB
```

#### Linux

```shell
lsusb -v
```

#### Windows

```powershell
Get-PnpDevice | Where-Object { $_.Class -eq 'USB' } | Format-Table Name, DeviceID, Manufacturer, Status, Description -AutoSize
```

### Monitoring generated keyboard and mouse events

The most simple (and cross-platform) way I have found is using `keyboard` and `mouse` Python modules.

Monitoring keyboard:

```shell
pip3 install keyboard
sudo python3 -m keyboard
```

Monitoring mouse:
* The latest published 'mouse' module doesn't support macOS, so use the latest version from GitHub

```shell
git clone https://github.com/boppreh/mouse
cd mouse
python3 -m mouse
```

## Supported Macro Keyboards

* Product ID: 0x8890
    * Vendor ID: 0x1189  (Trisat Industrial Co., Ltd.)
    * [amazon.co.jp/dp/B0CF5L8HP3](https://www.amazon.co.jp/dp/B0CF5L8HP3)

### Photos of Supported Keyboards

| 3x2 with 1 knob                       | 3x1 with 1 knob                       | 3×3 with 2 knobs                        |
| ------------------------------------- | ------------------------------------- | --------------------------------------- |
| ![keyboard-6-1](doc/keyboard-6-1.png) | ![keyboard-3-1](doc/keyboard-3-1.jpg) | ![keyboard-12-2](doc/keyboard-12-2.png) |
