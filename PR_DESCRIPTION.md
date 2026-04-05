# PR: Fix K884x LED CLI parsing and USB packet format

## Title
Fix K884x LED: CLI arg parsing and correct USB packet format

## Branch
`fix/k884x-led-cli` -> `kriomant:master`

## Description

Two bugs prevented LED control from working on 0x8840/0x8842 hardware. Both are fixed here.

### Bug 1: CLI argument parsing (#160)

The K884x LED command failed when specifying a mode with a color:

```
$ ch57x-keyboard-tool led 1 backlight cyan
Error: Invalid value 'backlight' for '<MODE>': Invalid LED mode
```

`LedArgs` used a single `LedMode` field with `value_parser`, but clap splits
`backlight cyan` into two separate positional arguments. The parser only received
`"backlight"` without the color.

**Fix:** Changed `LedArgs` to capture all trailing mode arguments as `Vec<String>`,
then joins them with a space before passing to the existing `LedMode` parser:

```rust
#[arg(num_args=1..)]
mode_args: Vec<String>,

fn mode(&self) -> Result<LedMode, String> {
    let combined = self.mode_args.join(" ");
    parse_led_mode(&combined)
}
```

### Bug 2: Incorrect USB packet format

Even with CLI parsing fixed, LED commands produced no visual response on hardware.
The `set_led` packet had the mode/color CODE byte at the wrong position (index 11
instead of 12), and incorrect filler bytes.

**Wrong packet (before):**
```
03 fe b0 LAYER 08 00 00 00 00 01 00 CODE 00 ...
                              ^^      ^^ index 11
```

**Correct packet (after), verified against USB captures and kamaaina/macropad_tool:**
```
03 fe b0 LAYER 08 00 00 00 00 00 01 00 CODE ...
                                 ^^     ^^ index 12
```

**Verified working on hardware** (0x1189:0x8840, 3×4 + 2 knobs):
```
$ ch57x-keyboard-tool led 0 backlight cyan   # all keys light up cyan ✓
$ ch57x-keyboard-tool led 1 press purple     # keys light up purple on press ✓
$ ch57x-keyboard-tool led 2 off              # LEDs off ✓
```

### Testing

- All existing tests pass (with updated byte assertions to match correct packet format)
- Added `test_led_args_from_cli`: verifies clap arg splitting for all mode/color combinations
- Added `test_led_packet_bytes`: asserts exact byte output matches confirmed USB captures
- Tested on real 0x8840 hardware: LED modes and colors work correctly

### References

- Fixes #160 (CLI parsing)
- USB packet format cross-referenced with kamaaina/macropad_tool and Wireshark captures
  shared in this issue thread by @Glutnix
