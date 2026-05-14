use std::str::FromStr;
use clap::Parser;
use nom::{IResult, branch::alt, bytes::complete::tag, character::complete::alpha1, combinator::{map, map_res, value}, sequence::preceded};
use serde_with::DeserializeFromStr;

use anyhow::{ensure, Result};

use super::{Key, Keyboard, Macro, KeyboardEvent, Modifier, MouseEvent, MouseAction, MouseModifier, send_message};

/// LED modes for the K8850 firmware (confirmed via hardware testing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, DeserializeFromStr)]
pub enum LedMode {
    Off,
    Static(Color),
    Reactive(Color),
    Ripple(Color),
    RainbowRows,
    RainbowCols,
}

impl LedMode {
    fn mode_byte(&self) -> u8 {
        match self {
            LedMode::Off => 0,
            LedMode::Static(_) => 1,
            LedMode::Reactive(_) => 2,
            LedMode::Ripple(_) => 3,
            LedMode::RainbowRows => 4,
            LedMode::RainbowCols => 5,
        }
    }

    fn color(&self) -> Color {
        match self {
            LedMode::Off => Color { r: 0, g: 0, b: 0 },
            LedMode::Static(c) | LedMode::Reactive(c) | LedMode::Ripple(c) => *c,
            LedMode::RainbowRows | LedMode::RainbowCols => Color { r: 0, g: 0, b: 0 },
        }
    }
}

impl FromStr for LedMode {
    type Err = nom::error::Error<String>;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        crate::parse::from_str(led_mode, s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl<'de> serde::Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        Color::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl serde::Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_str(&format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b))
    }
}

impl FromStr for Color {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "red" => return Ok(Color { r: 255, g: 0, b: 0 }),
            "green" => return Ok(Color { r: 0, g: 255, b: 0 }),
            "blue" => return Ok(Color { r: 0, g: 0, b: 255 }),
            "white" => return Ok(Color { r: 255, g: 255, b: 255 }),
            "yellow" => return Ok(Color { r: 255, g: 255, b: 0 }),
            "cyan" => return Ok(Color { r: 0, g: 255, b: 255 }),
            "magenta" => return Ok(Color { r: 255, g: 0, b: 255 }),
            "orange" => return Ok(Color { r: 255, g: 128, b: 0 }),
            "purple" => return Ok(Color { r: 128, g: 0, b: 255 }),
            _ => {}
        }
        if let Some(hex) = s.strip_prefix('#') {
            if hex.len() == 6 {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| e.to_string())?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| e.to_string())?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| e.to_string())?;
                return Ok(Color { r, g, b });
            }
        }
        Err(format!("unknown color '{}'. Use: red, green, blue, white, yellow, cyan, magenta, orange, purple, or #RRGGBB", s))
    }
}

fn hex_color(s: &str) -> IResult<&str, Color> {
    use nom::bytes::complete::take;
    let (s, _) = tag("#")(s)?;
    let (s, hex) = take(6usize)(s)?;
    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| nom::Err::Failure(nom::error::Error::new(s, nom::error::ErrorKind::HexDigit)))?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| nom::Err::Failure(nom::error::Error::new(s, nom::error::ErrorKind::HexDigit)))?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| nom::Err::Failure(nom::error::Error::new(s, nom::error::ErrorKind::HexDigit)))?;
    Ok((s, Color { r, g, b }))
}

fn color(s: &str) -> IResult<&str, Color> {
    alt((hex_color, map_res(alpha1, Color::from_str)))(s)
}

fn led_mode(s: &str) -> IResult<&str, LedMode> {
    alt((
        value(LedMode::Off, tag("off")),
        map(preceded(tag("static "), color), LedMode::Static),
        map(preceded(tag("reactive "), color), LedMode::Reactive),
        map(preceded(tag("ripple "), color), LedMode::Ripple),
        value(LedMode::RainbowRows, tag("rainbow-rows")),
        value(LedMode::RainbowCols, tag("rainbow-cols")),
        value(LedMode::RainbowRows, tag("rainbow")),
    ))(s)
}

fn parse_led_mode(s: &str) -> Result<LedMode, String> {
    crate::parse::from_str(led_mode, s).map_err(|e| format!("Invalid LED mode: {:?}", e))
}

#[derive(Parser, Debug)]
struct LedArgs {
    layer: u8,
    #[arg(value_parser=parse_led_mode)]
    mode: LedMode,
}

/// YAML structure for per-layer LED config. Parsed internally by the k8850 driver.
#[derive(Debug, serde::Deserialize)]
struct LedYamlConfig {
    mode: LedMode,
    colors: Vec<Vec<Color>>,
}

const NUM_KEY_SLOTS: usize = 16;

/// Driver for keyboards with product ID 0x8850 (e.g. XZKJ-16key_3knob).
///
/// The 8850 uses its own protocol for key bindings (`0xFD` command) rather
/// than the K884x's `0xFE` command. The format mirrors the FA read-response.
pub struct Keyboard8850 {
    buttons: u8,
    knobs: u8,
}

impl Keyboard for Keyboard8850 {
    /// Write a key binding using the 8850's FD protocol.
    ///
    /// The 8850 uses `0xFD` for key binding writes (NOT `0xFE` like k884x).
    /// The packet format mirrors the FA read-response format:
    ///
    /// ```text
    /// [0x03, 0xFD, key_id, layer, type, 0x00, binding_mode, 0x00, 0x00, ...]
    /// ```
    ///
    /// Shortcut mode (binding_mode=1): keycode at [9], modifier at [10]
    /// Macro mode (binding_mode=N): N triplets of [action, 0x00, 0x32]
    /// Media mode (binding_mode=2): consumer code at [9]
    fn bind_key(&self, layer: u8, key: Key, expansion: &Macro, output: &mut Vec<u8>) -> Result<()> {
        ensure!(layer < 3, "invalid layer index");

        let key_id = self.to_key_id(key)?;

        // Header: [0x03, 0xFD, key_id, layer(1-indexed), type, 0x00]
        let mut msg = vec![
            0x03,
            0xFD,
            key_id,
            layer + 1,
            expansion.kind(),
            0x00,
        ];

        match expansion {
            Macro::Keyboard(KeyboardEvent(_, presses)) => {
                // Single key, no modifiers: binding_mode=1 (compact format)
                if presses.len() == 1 && presses[0].modifiers.is_empty() {
                    let accord = &presses[0];
                    msg.push(0x01); // [6] binding_mode = shortcut
                    msg.push(0x00); // [7]
                    msg.push(0x00); // [8]
                    msg.push(accord.code.map_or(0, |c| c.value())); // [9] keycode
                    msg.push(0x00); // [10]
                } else {
                    // Everything else uses triplet format: [action, 00, 32] per entry.
                    // binding_mode = total entry count.
                    //
                    // Each Accord expands to: modifier triplets + keycode triplet
                    // e.g. "shift-b,r" -> [F2,00,32], [05,00,32], [15,00,32] (mode=3)
                    // e.g. "ctrl-d"    -> [F1,00,32], [07,00,32] (mode=2)
                    let entry_count: usize = presses.iter()
                        .map(|a| a.modifiers.len() + if a.code.is_some() { 1 } else { 0 })
                        .sum();
                    ensure!(entry_count <= 18, "macro sequence is too long ({} entries, max 18)", entry_count);

                    msg.push(entry_count as u8); // [6] binding_mode = entry count
                    msg.push(0x00); // [7]
                    msg.push(0x00); // [8]
                    for accord in presses.iter() {
                        for m in accord.modifiers.iter() {
                            msg.push(modifier_firmware_id(m));
                            msg.push(0x00);
                            msg.push(0x32); // 50ms delay
                        }
                        if let Some(code) = accord.code {
                            msg.push(code.value());
                            msg.push(0x00);
                            msg.push(0x32); // 50ms delay
                        }
                    }
                }
            }
            Macro::Media(code) => {
                // Media mode: binding_mode=2, consumer code at [9]
                let [low, _high] = (*code as u16).to_le_bytes();
                msg.push(0x02); // [6] binding_mode = media
                msg.push(0x00); // [7]
                msg.push(0x00); // [8]
                msg.push(low);  // [9] consumer code
            }
            Macro::Mouse(MouseEvent(action, modifier)) => {
                // 8850 mouse format (confirmed via Frida capture + PR #154 by yawor):
                // Position [5] in the full packet is the start of mouse_data (not 0x00 like keyboard).
                // Format: [1, 4, 0, 0, modifier, 0, 0, buttons, 0, 0, dx, 0, 0, dy, 0, 0, wheel]
                msg.pop(); // Remove the header's 0x00 at [5]; mouse_data provides its own bytes
                let mut mouse_data = [0u8; 17];
                mouse_data[0] = 1;
                mouse_data[1] = 4;
                mouse_data[4] = modifier.map_or(0, |m| mouse_modifier_firmware_id(m));

                match action {
                    MouseAction::Move(dx, dy) => {
                        mouse_data[10] = *dx as u8;
                        mouse_data[13] = *dy as u8;
                    }
                    MouseAction::Drag(buttons, dx, dy) => {
                        eprintln!("warning: drag may not work correctly on 8850 firmware (button hold + move sent as separate actions)");
                        mouse_data[7] = buttons.as_u8();
                        mouse_data[10] = *dx as u8;
                        mouse_data[13] = *dy as u8;
                    }
                    MouseAction::Click(buttons) => {
                        ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                        mouse_data[7] = buttons.as_u8();
                    }
                    MouseAction::Wheel(delta) => {
                        mouse_data[16] = *delta as u8;
                    }
                }

                msg.extend_from_slice(&mouse_data);
            }
        };

        send_message(output, &msg);

        // The 8850 requires a finalize packet after each key binding.
        send_message(output, &[0x03, 0xFD, 0xFE, 0xFF]);

        Ok(())
    }

    fn set_led(&mut self, args: &[String], output: &mut Vec<u8>) -> Result<()> {
        let led_args = LedArgs::try_parse_from(args)?;

        let target_layer = led_args.layer;
        ensure!(target_layer < 3, "Layer must be 0-2");

        let mode = led_args.mode.mode_byte();
        let color = led_args.mode.color();

        for layer in 0..3u8 {
            let mut msg = if layer == target_layer {
                let mut m = vec![0x03, 0xFE, 0xB0, layer, mode, color.r, color.g, color.b];
                for _ in 0..NUM_KEY_SLOTS {
                    m.extend_from_slice(&[color.r, color.g, color.b]);
                }
                m
            } else {
                let mut m = vec![0x03, 0xFE, 0xB0, layer, 0x00, 0x00, 0x00, 0x00];
                for _ in 0..NUM_KEY_SLOTS {
                    m.extend_from_slice(&[0x00, 0x00, 0x00]);
                }
                m
            };
            msg.truncate(64);
            send_message(output, &msg);
        }

        Ok(())
    }

    fn set_led_config(&self, layers: &[Option<serde_yaml::Value>], output: &mut Vec<u8>) -> Result<()> {
        for layer in 0..3u8 {
            let layer_config = layers.get(layer as usize).and_then(|l| l.as_ref());
            let mut msg = if let Some(value) = layer_config {
                let config: LedYamlConfig = serde_yaml::from_value(value.clone())
                    .map_err(|e| anyhow::anyhow!("invalid LED config for layer {}: {}", layer, e))?;
                let mode = config.mode.mode_byte();
                let flat_colors: Vec<Color> = config.colors.into_iter().flatten().collect();
                let base = flat_colors.first().copied()
                    .unwrap_or(Color { r: 0, g: 0, b: 0 });
                let mut m = vec![0x03, 0xFE, 0xB0, layer, mode, base.r, base.g, base.b];
                for i in 0..NUM_KEY_SLOTS {
                    let c = flat_colors.get(i).copied()
                        .unwrap_or(Color { r: 0, g: 0, b: 0 });
                    m.extend_from_slice(&[c.r, c.g, c.b]);
                }
                m
            } else {
                let mut m = vec![0x03, 0xFE, 0xB0, layer, 0x00, 0x00, 0x00, 0x00];
                for _ in 0..NUM_KEY_SLOTS {
                    m.extend_from_slice(&[0x00, 0x00, 0x00]);
                }
                m
            };
            msg.truncate(64);
            send_message(output, &msg);
        }
        Ok(())
    }

    fn preferred_endpoint() -> u8 where Self: Sized {
        0x04
    }
}

/// Map a Modifier to the 8850 firmware's modifier ID.
///
/// The 8850 firmware uses sequential IDs (not HID bitmasks):
///   F1=Ctrl, F2=Shift, F3=Alt, F4=Win
/// Confirmed via Frida captures of mini_keyboard.exe.
fn modifier_firmware_id(m: Modifier) -> u8 {
    match m {
        Modifier::Ctrl => 0xF1,
        Modifier::Shift => 0xF2,
        Modifier::Alt => 0xF3,
        Modifier::Win => 0xF4,
        Modifier::RightCtrl => 0xF5,
        Modifier::RightShift => 0xF6,
        Modifier::RightAlt => 0xF7,
        Modifier::RightWin => 0xF8,
    }
}

/// Map a MouseModifier to the 8850 firmware's modifier ID.
fn mouse_modifier_firmware_id(m: MouseModifier) -> u8 {
    match m {
        MouseModifier::Ctrl => 0xF1,
        MouseModifier::Shift => 0xF2,
        MouseModifier::Alt => 0xF3,
    }
}

impl Keyboard8850 {
    pub fn new(buttons: u8, knobs: u8) -> Result<Self> {
        ensure!(buttons <= 16 && knobs <= 3, "8850 supports up to 16 buttons and 3 knobs");
        Ok(Self { buttons, knobs })
    }

    /// 8850-specific key ID mapping (confirmed by hardware testing):
    /// - key_ids 1-16: 16 buttons (4x4 grid, row-major)
    /// - key_ids 17-19: knob0 (ccw, press, cw)
    /// - key_ids 20-22: knob1 (ccw, press, cw)
    /// - key_ids 23-25: knob2 (ccw, press, cw)
    fn to_key_id(&self, key: Key) -> Result<u8> {
        match key {
            Key::Button(n) if n >= self.buttons => Err(anyhow::anyhow!("invalid key index")),
            Key::Button(n) => Ok(n + 1),
            Key::Knob(n, _) if n >= self.knobs => Err(anyhow::anyhow!("invalid knob index")),
            Key::Knob(n, action) => Ok(self.buttons + 1 + 3 * n + (action as u8)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard::{Accord, Code, WellKnownCode, Modifier};
    use enumset::EnumSet;

    fn assert_msg(output: &[u8], msg_idx: usize, expected: &[u8]) {
        let start = msg_idx * 64;
        let actual = &output[start..start + expected.len()];
        assert_eq!(actual, expected, "message {} mismatch", msg_idx);
    }

    #[test]
    fn test_bind_key_shortcut() {
        // Bind button 0 on layer 0 to the letter "i" (HID 0x0C)
        let kb = Keyboard8850::new(16, 3).unwrap();
        let mut output = Vec::new();
        let macro_ = Macro::Keyboard(KeyboardEvent(
            Default::default(),
            vec![Accord { modifiers: EnumSet::empty(), code: Some(Code::WellKnown(WellKnownCode::I)) }],
        ));
        kb.bind_key(0, Key::Button(0), &macro_, &mut output).unwrap();

        // [03, FD, 01, 01, 01, 00, 01, 00, 00, 0C, 00]
        assert_msg(&output, 0, &[0x03, 0xFD, 0x01, 0x01, 0x01, 0x00, 0x01, 0x00, 0x00, 0x0C, 0x00]);
    }

    #[test]
    fn test_bind_key_shortcut_with_single_modifier() {
        // Bind button 0 on layer 0 to ctrl+a (single modifier -> binding_mode=2)
        let kb = Keyboard8850::new(16, 3).unwrap();
        let mut output = Vec::new();
        let macro_ = Macro::Keyboard(KeyboardEvent(
            Default::default(),
            vec![Accord { modifiers: Modifier::Ctrl.into(), code: Some(Code::WellKnown(WellKnownCode::A)) }],
        ));
        kb.bind_key(0, Key::Button(0), &macro_, &mut output).unwrap();

        // [03, FD, 01, 01, 01, 00, 02, 00, 00, F1, 00, 32, 04, 00, 32]
        assert_msg(&output, 0, &[0x03, 0xFD, 0x01, 0x01, 0x01, 0x00, 0x02, 0x00, 0x00, 0xF1, 0x00, 0x32, 0x04, 0x00, 0x32]);
    }

    #[test]
    fn test_bind_key_shortcut_with_multi_modifier() {
        // Bind button 0 on layer 0 to ctrl+alt+space (multi modifier -> binding_mode=3)
        let kb = Keyboard8850::new(16, 3).unwrap();
        let mut output = Vec::new();
        let mut mods = EnumSet::empty();
        mods.insert(Modifier::Ctrl);
        mods.insert(Modifier::Alt);
        let macro_ = Macro::Keyboard(KeyboardEvent(
            Default::default(),
            vec![Accord { modifiers: mods, code: Some(Code::WellKnown(WellKnownCode::Space)) }],
        ));
        kb.bind_key(0, Key::Button(0), &macro_, &mut output).unwrap();

        // mode=3 (2 modifiers + 1 keycode)
        // [03, FD, 01, 01, 01, 00, 03, 00, 00, F1, 00, 32, F3, 00, 32, 2C, 00, 32]
        assert_msg(&output, 0, &[0x03, 0xFD, 0x01, 0x01, 0x01, 0x00, 0x03, 0x00, 0x00, 0xF1, 0x00, 0x32, 0xF3, 0x00, 0x32, 0x2C, 0x00, 0x32]);
    }

    #[test]
    fn test_bind_key_shortcut_with_three_modifiers() {
        // Bind button 0 on layer 0 to ctrl+alt+win+F13 (3 modifiers -> binding_mode=4)
        let kb = Keyboard8850::new(16, 3).unwrap();
        let mut output = Vec::new();
        let mut mods = EnumSet::empty();
        mods.insert(Modifier::Ctrl);
        mods.insert(Modifier::Alt);
        mods.insert(Modifier::Win);
        let macro_ = Macro::Keyboard(KeyboardEvent(
            Default::default(),
            vec![Accord { modifiers: mods, code: Some(Code::WellKnown(WellKnownCode::F13)) }],
        ));
        kb.bind_key(0, Key::Button(0), &macro_, &mut output).unwrap();

        // mode=4 (3 modifiers + 1 keycode): F1(ctrl), F3(alt), F4(win), 68(F13)
        assert_msg(&output, 0, &[0x03, 0xFD, 0x01, 0x01, 0x01, 0x00, 0x04, 0x00, 0x00, 0xF1, 0x00, 0x32, 0xF3, 0x00, 0x32, 0xF4, 0x00, 0x32, 0x68, 0x00, 0x32]);
    }

    #[test]
    fn test_bind_key_macro_sequence() {
        // Bind button 0 on layer 0 to "a,b" (2-key macro)
        let kb = Keyboard8850::new(16, 3).unwrap();
        let mut output = Vec::new();
        let macro_ = Macro::Keyboard(KeyboardEvent(
            Default::default(),
            vec![
                Accord { modifiers: EnumSet::empty(), code: Some(Code::WellKnown(WellKnownCode::A)) },
                Accord { modifiers: EnumSet::empty(), code: Some(Code::WellKnown(WellKnownCode::B)) },
            ],
        ));
        kb.bind_key(0, Key::Button(0), &macro_, &mut output).unwrap();

        // Triplet format: mode=2 (2 entries), [04,00,32] [05,00,32]
        assert_msg(&output, 0, &[0x03, 0xFD, 0x01, 0x01, 0x01, 0x00, 0x02, 0x00, 0x00, 0x04, 0x00, 0x32, 0x05, 0x00, 0x32]);
    }

    #[test]
    fn test_bind_media_key() {
        // Bind button 0 on layer 0 to play/pause (0xCD)
        let kb = Keyboard8850::new(16, 3).unwrap();
        let mut output = Vec::new();
        let macro_ = Macro::Media(crate::keyboard::MediaCode::Play);
        kb.bind_key(0, Key::Button(0), &macro_, &mut output).unwrap();

        // [03, FD, 01, 01, 02, 00, 02, 00, 00, CD]
        assert_msg(&output, 0, &[0x03, 0xFD, 0x01, 0x01, 0x02, 0x00, 0x02, 0x00, 0x00, 0xCD]);
    }

    #[test]
    fn test_bind_key_includes_finalize() {
        // Every bind_key call should append a finalize packet [03 FD FE FF]
        let kb = Keyboard8850::new(16, 3).unwrap();
        let mut output = Vec::new();
        let macro_ = Macro::Keyboard(KeyboardEvent(
            Default::default(),
            vec![Accord { modifiers: EnumSet::empty(), code: Some(Code::WellKnown(WellKnownCode::A)) }],
        ));
        kb.bind_key(0, Key::Button(0), &macro_, &mut output).unwrap();

        // Should be 2 messages: the key binding + the finalize
        assert_eq!(output.len(), 2 * 64);
        assert_msg(&output, 1, &[0x03, 0xFD, 0xFE, 0xFF]);
    }

    #[test]
    fn test_led_static_red() {
        let mut kb = Keyboard8850::new(16, 3).unwrap();
        let mut output = Vec::new();
        kb.set_led(&["led".to_string(), "0".to_string(), "static red".to_string()], &mut output).unwrap();

        assert_eq!(output.len(), 3 * 64);
        let led0 = &output[0..64];
        assert_eq!(led0[0], 0x03);
        assert_eq!(led0[1], 0xFE);
        assert_eq!(led0[2], 0xB0);
        assert_eq!(led0[3], 0x00);
        assert_eq!(led0[4], 0x01);
        assert_eq!(led0[5], 0xFF);
        assert_eq!(led0[6], 0x00);
        assert_eq!(led0[7], 0x00);
    }

    #[test]
    fn test_led_off() {
        let mut kb = Keyboard8850::new(16, 3).unwrap();
        let mut output = Vec::new();
        kb.set_led(&["led".to_string(), "0".to_string(), "off".to_string()], &mut output).unwrap();
        let led0 = &output[0..64];
        assert_eq!(led0[4], 0x00);
    }

    #[test]
    fn parse_led_modes() {
        assert_eq!("off".parse(), Ok(LedMode::Off));
        assert_eq!("static red".parse(), Ok(LedMode::Static(Color { r: 255, g: 0, b: 0 })));
        assert_eq!("reactive blue".parse(), Ok(LedMode::Reactive(Color { r: 0, g: 0, b: 255 })));
        assert_eq!("ripple green".parse(), Ok(LedMode::Ripple(Color { r: 0, g: 255, b: 0 })));
        assert_eq!("rainbow".parse(), Ok(LedMode::RainbowRows));
        assert_eq!("rainbow-rows".parse(), Ok(LedMode::RainbowRows));
        assert_eq!("rainbow-cols".parse(), Ok(LedMode::RainbowCols));
    }

    #[test]
    fn parse_hex_color() {
        assert_eq!(Color::from_str("#FF8000"), Ok(Color { r: 255, g: 128, b: 0 }));
        assert_eq!(Color::from_str("#000000"), Ok(Color { r: 0, g: 0, b: 0 }));
    }

}
