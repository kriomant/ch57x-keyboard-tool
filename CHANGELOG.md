# Changelog

## Unreleased

- Fix handling fourth knob on 8850

## 1.6.0

- Described udev configuration to avoid using sudo
- Added aarch64 build target for devices like Raspberry Pi (#151)
- Added aliases to keys changing screen brightness on Mac
- Mouse move action
- Mouse drag action
- Alternative syntax for click action
- New 'wheel' action
- Added support for fourth knob on 8850 model
- LED control for 884x

## 1.5.4

- Reverted key index check fix for k884x due to issues

## 1.5.3

- `--version` option to display version information (#130)
- Key index check for k884x keyboards (#97)

## 1.5.2

- Support for product ID 0x8850 (#122)
- Send command to persist binding for 884x keyboards (#121)

## 1.5.1

- Support for 4x1 device without knobs (#118)
- Send command to persist binding for 884x keyboards (#121)

## 1.5.0

- Removed mentions of key scan codes
- Simplified usage and enhanced documentation

## 1.4.4

- Fixed 8890 keyboard support

## 1.4.2

- Support for modifier-only keys on k884x
- Accept path to config file instead of reading from stdin
- Increased macro length limit to 18 for 884x keyboards (#80)

## 1.4.1

- Support for product ID 0x8840 (#78)
- Fixed 8890 endpoint autodetection

## 1.4.0

- Support for product IDs 8840 and 8842 (#62)
- Added MIT license

## 1.2.4

- Ability to specify endpoint address

## 1.2.3

- PowerShell usage instructions
- USB info dumping on errors for better diagnostics

## 1.2.2

- Note about 'sudo' usage in README (#37)
- Note about using tool for automation
- Validation checks for restrictions of 3x1+1 keyboard

## 1.2.1

- Mac universal binary build fix

## 1.2.0

- 'stop' media key (#14)
- Favorites and Calculator keys (#22)
- Universal binary support for Mac

## 1.1.0

- USBDK support for Windows
- Command to validate config file
- Command to show supported keys
- Support for arbitrary key scan codes
- Documentation for 3x1 keyboard (#6)
- Added 3x2 + 1 keyboard to tested keyboards list (#9)

## 1.0.0

- Initial release of keyboard mapper
- Configuration loading from stdin
- Support for macros and key combinations
- Mouse event support (clicks with modifiers and multiple buttons)
- Knob support
- Media event support
- LED mode switching command
- Device selection when multiple devices are connected
- Support for F13-F24 keys
- Alternative Mac-specific modifier names