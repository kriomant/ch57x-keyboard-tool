# Available Actions

This document describes all available actions that can be assigned to keys and knobs on CH57x macro keyboards.

## Table of Contents

- [Overview](#overview)
- [Keyboard Actions](#keyboard-actions)
  - [Simple Keys](#simple-keys)
  - [Key Combinations](#key-combinations)
  - [Key Sequences](#key-sequences)
  - [Modifiers](#modifiers)
- [Mouse Actions](#mouse-actions)
  - [Mouse Click](#mouse-click)
  - [Mouse Move](#mouse-move)
  - [Mouse Drag](#mouse-drag)
  - [Mouse Wheel](#mouse-wheel)
- [Media Keys](#media-keys)
- [Examples](#examples)

## Overview

Each key or knob on your macro keyboard can be programmed to perform different actions. Actions are defined in the YAML configuration file and can be:

- **Keyboard actions**: emulate keypresses, key combinations, and sequences
- **Mouse actions**: control mouse movement, clicks, and scrolling
- **Media keys**: control media playback and system functions

## Keyboard Actions

### Simple Keys

Press a single key without modifiers:

- *a* — 'A letter' key (see note below)
- slash — '/' key
- *enter*
- *esc*
- *space*
- *f1*

To see a complete list of supported key names, run:
```shell
./ch57x-keyboard-tool show-keys
```

It is also possible to emulate press of key for which no name is given,
use HID code in angle brackets like this:

- *<101>*

See https://www.usb.org/sites/default/files/documents/hut1_12v2.pdf (section 10)
for HID usage code list.

**Important**:

When you use letters, you specify *key* to press, not *character* to produce. If you use keyboard layout other than QWERTY, pressing 'a' key may in fact produce another character, not 'a'.

### Key Combinations

Use modifiers with keys by joining them with a hyphen (`-`):

- *ctrl-c*
- *cmd-v*
- *alt-tab*
- *shift-a*
- *ctrl-alt-del*

### Modifiers

Available keyboard modifiers:

| Modifier | Aliases | Description
|----------|---------|-------------
| `ctrl`   | -       | Control key
| `rctrl`  | -       | Right Control
| `shift`  | -       | Shift key
| `rshift` | -       | Right Shift
| `alt`    | `opt`   | Alt (Windows/Linux) / Option (macOS)
| `ralt`   | `ropt`  | Right Alt / Right Option
| `win`    | `cmd`   | Windows (Windows/Linux) / Command (macOS)
| `rwin`   | `rcmd`  | Right Windows / Right Command

You can combine multiple modifiers:

- *ctrl-shift-a*
- *cmd-alt-esc*

### Key Sequences (Chords)

Execute multiple key presses in sequence by separating them with commas (`,`):

- *h,e,l,l,o* — types "hello"
- *ctrl-a,ctrl-c* — select all, then copy
- *win-r,c,m,d,enter* — open Run dialog and type "cmd"

Up to five combinations may be used.

**Note for 3x1 keyboards**: Key modifiers are only supported for the first key in a sequence. You can use `ctrl-alt-del,1,2` but not `ctrl-alt-del,alt-1,2`.

## Mouse Actions

Mouse actions allow you to control the mouse cursor and buttons directly from your keyboard.

Supported mouse buttons:

- *left*
- *right*
- *middle*

You may use combination of buttons: *left+right*.

Mouse actions support the following modifiers:

- *ctrl* - Control key
- *shift* - Shift key
- *alt* - Alt key

**Note**: Unlike keyboard actions, mouse actions only support these three modifiers (no Win/Cmd key).

### Mouse Click

- *click(left)* — left button click
- *click(left+right)* — two buttons click

You can also use modifiers with mouse clicks:

- *ctrl-click(left)*
- *shift-click(right)*

There are aliases for clicks:

- *click* is *click(left)*
- *rclick* is *click(right)*
- *mclick* is *click(middle)*

### Mouse Move

Move the mouse cursor by a relative offset:

- *move(10,0)* — move 10 pixels right
- *move(-10,0)* — move 10 pixels left
- *move(0,10)* — move 10 pixels down
- *move(5,5)* — Move 5 pixels right and 5 pixels down

Valid range for movement: **-128 to 127 pixels** in each direction.

You can add modifiers:

- *ctrl-move(10,5)* — move while holding Ctrl

### Mouse Drag

Drag with mouse buttons pressed:

- *drag(left,10,0)* — drag 10 pixels right with left button
- *drag(right,-5,5)* — drag with right button
- *drag(left+right,0,10)* — drag with both buttons

With modifiers:

- *shift-drag(left,10,10)* — drag while holding Shift

### Mouse Wheel

Rotate the mouse wheel:

- *wheel(1)* — rotate up by 1 unit
- *wheel(-1)* — rotate down by 1 unit
- *wheel(3)* — rotate up by 3 units

Valid range: **-128 to 127**.

**Note**: Unit given to *wheel* action doesn't correspond to anything, it is neither line nor pixel. And it is also not linear: on my setup *wheel(100)* scrolls just 3 times more than *wheel(1)*.

With modifiers:

- *ctrl-wheel(1)* — rotate while holding Ctrl (often zooms)

Aliases:

- *wheelup* is *wheel(1)*
- *wheeldown* is *wheel(-1)*

## Media Keys

Control media playback and system functions:

| Key | Description |
|-----|-------------|
| `next` | Next track |
| `previous` or `prev` | Previous track |
| `stop` | Stop playback |
| `play` | Play/Pause |
| `mute` | Mute audio |
| `volumeup` | Increase volume |
| `volumedown` | Decrease volume |
| `favorites` | Open favorites |
| `calculator` | Open calculator |
| `screenlock` | Lock screen |

Example:

- *play*
- *volumeup*
- *next*

## Notes

### Custom Keyboard Layouts

When using custom keyboard layouts (like Dvorak), you must specify the key as it appears on a QWERTY layout, not the character it produces. The keyboard emulates physical key presses, not character input.

### Getting Help

- Run `./ch57x-keyboard-tool show-keys` to see all supported key names
- Run `./ch57x-keyboard-tool validate your-config.yaml` to check your configuration
- See `example-mapping.yaml` for a complete configuration example