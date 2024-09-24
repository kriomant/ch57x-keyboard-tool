# ch57x-keyboard-tool Macro Keyboard Configuration Utility

![Last Commit Shields.io](https://img.shields.io/github/last-commit/kriomant/ch57x-keyboard-tool?style=for-the-badge) ![Release Workflow Badge](https://github.com/kriomant/ch57x-keyboard-tool/actions/workflows/release.yml/badge.svg)

## Table of Contents <!-- omit in toc -->

* [What is this?](#what-is-this)
    * [Supported keyboards](#supported-keyboards)
* [Installation](#installation)
    * [Prebuilt release](#prebuilt-release)
    * [Build it yourself](#build-it-yourself)
* [Usage](#usage)
    * [Commands and options](#commands-and-options)
    * [Create configuration file](#create-configuration-file)
    * [All possible keys](#all-possible-keys)
    * [Validate the config file](#validate-the-config-file)
    * [Upload the config to the keyboard](#upload-the-config-to-the-keyboard)
    * [Change LED configuration](#change-led-configuration)
    * [Windows / PowerShell](#windows--powershell)
* [FAQ](#faq)
    * [How to do … on key press?](#how-to-do--on-key-press)
    * [Can you implement … feature?](#can-you-implement--feature)
    * [Why do the media controls always trigger in my browser and not in my music app?](#why-do-the-media-controls-always-trigger-in-my-browser-and-not-in-my-music-app)
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
* [All possible keys](#all-possible-keys)

## What is this?

This keyboard configuration utility is for programming small keyboards, such as the one shown below:

![Picture of keyboard-12-2](doc/keyboard-12-2.png)

Such macro keyboards are popular on AliExpress, and sellers often include software for programming, but:
* It requires Windows
* It is very ugly and inconvenient
* It can only program one key at a time
* It does not expose all keyboard features

There are several modifications of such keyboards with different numbers of buttons and knobs (see the [photos of supported keyboards](#photos-of-supported-keyboards)) and with/without Bluetooth.

Both wired and wireless keyboards are supported.  
⚠️ However, the keyboard must be connected to the computer with a USB cable when programming.

### Supported keyboards

This utility has been reported to work with:
* 3×4 with 2 knobs (Bluetooth version)
* 3×3 with 2 knobs
* 3x2 with 1 knob
* 3x1 with 1 knob with [limitations](#3x1-keys--1-knob-keyboard-limitations)

Keyboard with following vendor/product IDs are supported: `1189:8890`, `1189:8840`, `1189:8842` (hexadecimal).

For more details, refer to the [Supported Macro Keyboards](#supported-macro-keyboards) section.

## Installation

There are two ways to download the keyboard utility: getting a prebuilt release or building it yourself.

### Prebuilt release

Simply download the [latest release from GitHub](https://github.com/kriomant/ch57x-keyboard-tool/releases).

### Or build it yourself

1. Install the *cargo* utility using [rustup](https://rustup.rs/):
    * Brew: `brew install rustup-init && rustup-init`
    * Linux: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
    * Windows: Download and run [rustup-init.exe](https://win.rustup.rs/)
2. Execute `cargo install ch57x-keyboard-tool`.

### If you are on Windows

Install [USBDK](https://github.com/daynix/UsbDk/releases).

## Usage

1. Connect the keyboard to the computer with a USB cable.
2. Create a configuration file based on the provided [example-mapping.yaml](example-mapping.yaml).
3. Validate the configuration file.
4. Upload the configuration to the keyboard.
5. Done! 🎉

### Commands and options

```shell
ch57x-keyboard-tool [OPTIONS] <COMMAND>
```

Commands and their descriptions:

| Command                | Description                                               |
| ---------------------- | --------------------------------------------------------- |
| `show-keys`            | Display a list of all supported keys and modifiers        |
| `validate`             | Validate key mappings config from stdin                   |
| `upload`               | Upload key mappings from stdin to the device              |
| `led`                  | Select LED backlight mode                                 |
| `help`, `-h`, `--help` | Print this message or the help of the given subcommand(s) |

### Create configuration file

Edit existing `example-mapping.yaml` or (better) save modified copy under different name.

Example config file has extensive documentation inside.

You may also get list of supported key names using:

```shell
./ch57x-keyboard-tool show-keys
```

### All possible keys

Each entry is either a sequence of chords or a single chord, or a mouse event.

You can combine keys into chords by joining them with a dash `-`. Example: `ctrl-alt-del` or `ctrl-shift-7`. 

You can also combine up to five chords into a sequence by joining them with a comma `,`. Example: `ctrl-a,ctrl-c`. 

Arbitrary HID usage codes (decimal) may be given like this: `<101>`. See [section 10](https://www.usb.org/sites/default/files/documents/hut1_12v2.pdf) for the HID usage code list.

Mouse events are clicks (`click/lclick`, `rclick`, `mclick`) or wheel events (`wheelup`, `wheeldown`) with one optional modifier, only `ctrl`, `shift` and `alt` are supported (`ctrl-wheeldown`). Clicks may combine several buttons, like this: `click+rclick`.

Media keys cannot be combined with normal keys and modifiers.

- **Modifiers**: `ctrl`, (`alt` or `opt`), (`cmd` or `win`), `shift`, `rctrl`, (`rcmd` or `rwin`), (`ropt` or `ralt`), `rshift`

- **Media keys**: `next`, (`prev` or `previous`), `stop`, `play`, `pause`, `mute`, `volumeup`, `volumedown`, `favorites`, `calculator`, `screenlock`

- **Letters**: All the letters of the alphabet `a` through `z`

- **Numbers**: All the numbers `0` through `9` and all the numbers `numpad0` through `numpad9`

- **Function keys**: `f1` through `f24`

- **Numpad**: `numlock`, `numpadslash`, `numpadasterisk`, `numpadminus`, `numpadplus`, `numpadenter`, `numpaddot`, `numpadequal`

- **Navigation keys**: `insert`, `home`, `pageup`, `delete`, `end`, `pagedown`, `right`, `left`, `down`, `up`

- **Editing and formatting**: `backspace`, `tab`, `space`, `enter`, `escape`, `capslock`, `printscreen`, `scrolllock`, `pause`

- **Special characters**: `minus`, `equal`, `leftbracket`, `rightbracket`, `backslash`, `nonusbackslash`, `nonushash`

- **Punctuation**: `semicolon`, `quote`, `grave`, `comma`, `dot`, `slash`

- **System and application**: `application`, `power`

- **Mouse**: (`click` or `lclick`), `mclick`, `rclick`, `wheelup`, `wheeldown`

### Validate the config file

```shell
./ch57x-keyboard-tool validate your-config.yaml
```

### Upload the config to the keyboard

```shell
./ch57x-keyboard-tool upload your-config.yaml
```

Use 'sudo' if you get 'Access denied (insufficient permissions)':

```shell
sudo ./ch57x-keyboard-tool upload your-config.yaml
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

## FAQ

### How to do ... on key press?

A common question/request is about automation, such as "How to run a script?", "emulate several keys", or "how to trigger an action with a key press?"

This tool does just one job: **writes your key bindings into the keyboard** and then exits.  
It does not listen for key presses.
Automation based on key presses is not within the scope of this utility tool.

If you seek any automation, use third-party automation tools like [BetterTouchTool](https://folivora.ai/).

1. Choose a chord you do not usually use (like `alt-ctrl-shift-1`).
2. Assign the chord to a key.
3. Use a third-party automation tool to listen for this chord and have it perform the desired action.
4. Done! 🎉

### Can you implement ... feature?

I don't have detailed datasheet for these keyboards. So I can say whether something can implemented until you show me any software that can do it. Then it is teoretically possible to replicate behavior.

However, doing it requires either exact keyboard model in my hands or you to performa reverse engeneering.

### Why do the media controls always trigger in my browser and not in my music app?

This is a common issue with media keys and browser having a higher priority on those keys than the music app, for example Spotify. To fix this, you can switch off a certain flag in your browser. 

In Chrome, go to `chrome://flags/#hardware-media-key-handling` and disable the flag. This should work on any Chromium-based browser like Edge, Brave or Opera.

In Firefox, go to `about:config` and set `media.hardwaremediakeys.enabled` to `false`.

This should now allow your music app to receive the media key presses.

## Notes

### Number of layers

All keyboards I have seen have three layers (three key configurations which may be switched).
However, if your keyboard does not support layer switching, just keep a single layer in the configuration file.

### Custom keyboard layouts

Note that you specify key to emulate press for, not character which is produced by pressing it.
So if you use a custom keyboard layout, like [Dvorak](https://en.wikipedia.org/wiki/Dvorak_keyboard_layout), you have to see how required key is labelled in QWERTY layout.

### 3x1 keys + 1 knob keyboard limitations

This modification does support key modifiers (like `ctrl-`, `alt-`, and `cmd-`) for the first key in sequence only.

So, you can use: `ctrl-alt-del,1,2`, but not `ctrl-alt-del,alt-1,2`.

### macOS vs Windows keyboard keys

A friendly reminder that some keys have different names on macOS and Windows.  
These keys have aliases for both platforms, you may use them interchangeably.

| Key Name          | macOS Key | Windows Key |
| ----------------- | --------- | ----------- |
| Command / Windows | `cmd`     | `win`       |
| Option / Alt      | `opt`     | `alt`       |

### Advanced options

Advanced options, you don't have to use this normally:

| Option                      | Description                 | Notes            |
| --------------------------- | --------------------------- | ---------------- |
| `--vendor-id <VENDOR_ID>`   | Vendor ID of the keyboard   | Default: `4489`  |
| `--product-id <PRODUCT_ID>` | Product ID of the keyboard  | Default: `34960` |
| `--address <ADDRESS>`       | Address of the keyboard     |                  |

**⚠️ The ability to override the vendor/product ID does not mean that you can use this utility to program arbitrary keyboards!**

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

The simplest and cross-platform way to monitor keyboard and mouse events is using the `keyboard` and `mouse` Python modules.

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

* Product ID: 0x8890, 0x8840
    * Vendor ID: 0x1189  (Trisat Industrial Co., Ltd.)
    * [amazon.co.jp/dp/B0CF5L8HP3](https://www.amazon.co.jp/dp/B0CF5L8HP3)

### Photos of supported keyboards

| 3x2 with 1 knob                       | 3x2 with 1 knob                        | 3x1 with 1 knob                       | 3×3 with 2 knobs                        |
|---------------------------------------|----------------------------------------|---------------------------------------|-----------------------------------------|
| ![keyboard-6-1](doc/keyboard-6-1.png) | ![keyboard-6-1](doc/keyboard-6-1a.png) | ![keyboard-3-1](doc/keyboard-3-1.jpg) | ![keyboard-12-2](doc/keyboard-12-2.png) |
| 4x3 with 3 knobs                      |
| ![keyboard-4-3](doc/keyboard-4-3.png) |