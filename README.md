# ch57x-keyboard-tool Macro Keyboard Configuration Utility

![Last Commit Shields.io](https://img.shields.io/github/last-commit/kriomant/ch57x-keyboard-tool?style=for-the-badge) ![Release Workflow Badge](https://github.com/kriomant/ch57x-keyboard-tool/actions/workflows/release.yml/badge.svg)

## Table of Contents <!-- omit in toc -->

* [What is this?](#what-is-this)
    * [Supported keyboards](#supported-keyboards)
* [Installation](#installation)
    * [Get prebuilt release](#get-prebuilt-release)
    * [Build it yourself](#build-it-yourself)
* [Usage](#usage)
    * [Commands and options](#commands-and-options)
    * [Validate the config file](#validate-the-config-file)
    * [Upload the config to the keyboard](#upload-the-config-to-the-keyboard)
    * [Change LED configuration](#change-led-configuration)
    * [Windows / PowerShell](#windows--powershell)
* [Automation](#automation)
* [Notes](#notes)
    * [Number of layers](#number-of-layers)
    * [Custom keyboard layouts](#custom-keyboard-layouts)
    * [3x1 keys + 1 knob keyboard limitations](#3x1-keys--1-knob-keyboard-limitations)
    * [macOS vs Windows keyboard keys](#macos-vs-windows-keyboard-keys)
* [Diagnostics](#diagnostics)
    * [How to find and list connected USB devices](#how-to-find-and-list-connected-usb-devices)
        * [macOS](#macos)
        * [Linux](#linux)
        * [Windows](#windows)
    * [Monitoring generated keyboard and mouse events](#monitoring-generated-keyboard-and-mouse-events)
* [Supported macro keyboards](#supported-macro-keyboards)
    * [Photos of supported keyboards](#photos-of-supported-keyboards)

## What is this?

This is a utility for programming small keyboards like this one:

![Picture of keyboard-12-2](doc/keyboard-12-2.png)

Such macro keyboards are popular on AliExpress, and sellers often include software for programming, but:
* requires Windows
* is very ugly and inconvenient
* can only program one key at a time
* do not expose all keyboard features

There are several modifications of such keyboards with different numbers of buttons and knobs (See the [photos of supported keyboards](#photos-of-supported-keyboards)) and with/without Bluetooth.

Both wired and wireless keyboards are supported.  
⚠️ However, the keyboard must be connected to the computer with the USB cable when programming.

### Supported keyboards

This utility was reported to work with:
* 3×4 with 2 knobs (Bluetooth version)
* 3×3 with 2 knobs
* 3x2 with 1 knob
* 3x1 with 1 knob with [limitations](#3x1-keys--1-knob-keyboard-limitations))

All these keyboards share the same vendor/product IDs: `1189:8890` (hexadecimal).
It is possible to override used vendor/product ID, but it is usually unnecessary.
Use it only if you find the same-looking keyboard with other vendor/product ID,
I haven't seen such.

For more details, refer to the [Supported Macro Keyboards](#supported-macro-keyboards) section.

**⚠️ Ability to override vendor/product ID doesn't mean that you can use this software for programming arbitrary keyboards!**

## Installation

There are two ways to download the keyboard utility: prebuilt release or build it yourself.

### Get the prebuilt release

Simply download the [latest release from GitHub](https://github.com/kriomant/ch57x-keyboard-tool/releases)

### Build it yourself

1. Install the *cargo* utility using [rustup](https://rustup.rs/)
    * Brew: `brew install rustup-init && rustup-init`
    * Linux: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
    * Windows: download and run [rustup-init.exe](https://win.rustup.rs/)
1. Execute `cargo install ch57x-keyboard-tool`.

## Usage

**Note**: Windows users need to install [USBDK](https://github.com/daynix/UsbDk/releases) first.

1. Connect the keyboard to the computer with the USB cable.
1. Create a configuration file based on the provided [example-mapping.yaml](example-mapping.yaml).
    * The example config file has extensive documentation inside.
1. Validate the configuration file.
1. Upload the configuration to the keyboard.
1. Done! 🎉

### Commands and options

```shell
ch57x-keyboard-tool [OPTIONS] <COMMAND>
```

| Command                | Description                                               |
| ---------------------- | --------------------------------------------------------- |
| `show-keys`            | Display a list of all supported keys and modifiers        |
| `validate`             | Validate key mappings config on stdin                     |
| `upload`               | Upload key mappings from stdin to device                  |
| `led`                  | Select LED backlight mode                                 |
| `help`, `-h`, `--help` | Print this message or the help of the given subcommand(s) |

| Option                      | Description                | Notes            |
| --------------------------- | -------------------------- | ---------------- |
| `--vendor-id <VENDOR_ID>`   | Vendor ID of the keyboard  | Default: `4489`  |
| `--product-id <PRODUCT_ID>` | Product ID of the keyboard | Default: `34960` |
| `--address <ADDRESS>`       | Address of the keyboard    |                  |

### Validate the config file

```shell
./ch57x-keyboard-tool validate < your-config.yaml
```

### Upload the config to the keyboard

```shell
./ch57x-keyboard-tool upload < your-config.yaml
```

Use 'sudo' if you get 'Access denied (insufficient permissions)':

```shell
sudo ./ch57x-keyboard-tool upload < your-config.yaml
```

### Change LED configuration

If your keyboard supports it, you can change the LED configuration:

```shell
# Turn off the LED
./ch57x-keyboard-tool led 0

# Set the LED to the first mode (likely "Steady on")
./ch57x-keyboard-tool led 1
```

### Windows / PowerShell

Use `Get-Content` for input redirection:

```shell
Get-Content your-config.yaml | ./ch57x-keyboard-tool validate
```

## Automation

A common question/requests are about automation such as "How to run a script?", "emulate several keys", or "how to trigger an action with a key press?"

This tool does just one job: **writes your key bindings into a keyboard** and then exists.  
It does not listen for key presses.
Automation based on key presses is not in the scope of this utility tool.

If you want any automation, use third-party automation tools like [BetterTouchTool](https://folivora.ai/) or 

1. Choose a chord you do not usually use (like `alt-ctrl-shift-1`)
1. Assign the chord to a key
2. Use a third-party automation tool to listen for this chord and have it perform the desired action
3. Done! 🎉

## Notes

### Number of layers

All keyboards I have seen have three layers (three key configurations which may be switched).
But, if your keyboard does not support layer switching, just keep a single layer in the configuration file.

### Custom keyboard layouts

If you use a custom keyboard layout, like [Dvorak](https://en.wikipedia.org/wiki/Dvorak_keyboard_layout), you will need to write the keyboard key's [scancode](https://en.wikipedia.org/wiki/Scancode) in the configuration file (not the character that is produced).

So use the QWERTY letter of the keyboard key you want to press.

### 3x1 keys + 1 knob keyboard limitations

This modification does support key modifiers (like `ctrl-`, `alt-`, and `cmd-`) for the first key in sequence only.

So you can use: `ctrl-alt-del,1,2`, but not `ctrl-alt-del,alt-1,2`.

### macOS vs Windows keyboard keys

Friendly reminder that some keys have different names on macOS and Windows.  
Make sure to use the correct key names in your configuration file.

| Key Name          | macOS Key | Windows Key |
| ----------------- | --------- | ----------- |
| Command / Windows | `cmd`     | `win`       |
| Option / Alt      | `cmd`     | `alt`       |

## Diagnostics

When reporting an issue, please include diagnostics such as the list of attached USB devices and the output of the `keyboard` and `mouse` monitoring tools.

### How to find and list connected USB devices

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

The `keyboard` and `mouse` Python modules is the simplest and cross-platform way to monitor keyboard and mouse events.

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

## Supported macro keyboards

* Product ID: 0x8890
    * Vendor ID: 0x1189  (Trisat Industrial Co., Ltd.)
    * [amazon.co.jp/dp/B0CF5L8HP3](https://www.amazon.co.jp/dp/B0CF5L8HP3)

### Photos of supported keyboards

| 3x2 with 1 knob                       | 3x1 with 1 knob                       | 3×3 with 2 knobs                        |
| ------------------------------------- | ------------------------------------- | --------------------------------------- |
| ![keyboard-6-1](doc/keyboard-6-1.png) | ![keyboard-3-1](doc/keyboard-3-1.jpg) | ![keyboard-12-2](doc/keyboard-12-2.png) |
