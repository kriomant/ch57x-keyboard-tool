use std::str::FromStr;

use anyhow::{ensure, Result};
use clap::Parser;
use nom::{IResult, branch::alt, bytes::complete::tag, character::complete::alpha1, combinator::{map, map_res, value}, sequence::preceded};

use super::{Key, Keyboard, Macro, send_message};
use super::k884x::Keyboard884x;

/// LED modes for the K8850 firmware (confirmed via hardware testing).
///
/// The K8850 uses per-key RGB with an explicit mode byte, unlike the K884x
/// which encodes mode and color into a single byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedMode {
    /// LEDs off (mode 0)
    Off,
    /// Static color - all keys show the specified color (mode 1)
    Static(Color),
    /// Reactive - keys light up on press (mode 2)
    Reactive(Color),
    /// Ripple - ripple effect from pressed key (mode 3)
    Ripple(Color),
    /// Rainbow chase across keys (mode 4, color ignored)
    Rainbow,
}

impl LedMode {
    fn mode_byte(&self) -> u8 {
        match self {
            LedMode::Off => 0,
            LedMode::Static(_) => 1,
            LedMode::Reactive(_) => 2,
            LedMode::Ripple(_) => 3,
            LedMode::Rainbow => 4,
        }
    }

    fn color(&self) -> Color {
        match self {
            LedMode::Off => Color { r: 0, g: 0, b: 0 },
            LedMode::Static(c) | LedMode::Reactive(c) | LedMode::Ripple(c) => *c,
            LedMode::Rainbow => Color { r: 0, g: 0, b: 0 },
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

impl FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // Try named colors first
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
        // Try hex: #RRGGBB
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

fn named_color(s: &str) -> IResult<&str, Color> {
    map_res(alpha1, Color::from_str)(s)
}

fn led_mode(s: &str) -> IResult<&str, LedMode> {
    alt((
        value(LedMode::Off, tag("off")),
        map(preceded(tag("static "), named_color), LedMode::Static),
        map(preceded(tag("reactive "), named_color), LedMode::Reactive),
        map(preceded(tag("ripple "), named_color), LedMode::Ripple),
        value(LedMode::Rainbow, tag("rainbow")),
    ))(s)
}

fn parse_led_mode(s: &str) -> Result<LedMode, String> {
    crate::parse::from_str(led_mode, s).map_err(|e| format!("Invalid LED mode: {:?}", e))
}

#[derive(Parser, Debug)]
struct LedArgs {
    /// Layer to set the LED (0-2)
    layer: u8,

    /// LED mode and color (e.g. "static red", "reactive #FF0000", "rainbow", "off")
    #[arg(value_parser=parse_led_mode)]
    mode: LedMode,
}

/// Number of addressable key slots in the LED packet.
const NUM_KEY_SLOTS: usize = 16;

/// Driver for keyboards with product ID 0x8850 (e.g. XZKJ-16key_3knob).
///
/// Key bindings use the same protocol as the K884x family, but the LED
/// protocol is completely different:
///
/// 1. An INIT packet `[0x03, 0xFB, 0xFB, 0xFB, ...]` must be sent first
///    to put the device into configuration mode.
///
/// 2. LED data uses per-key RGB with an explicit mode byte:
///    `[0x03, 0xFE, 0xB0, layer, mode, R, G, B, <16 × RGB per key>]`
///
///    - layer: 0-indexed (0, 1, 2)
///    - mode: 0=off, 1=static, 2=reactive, 3=ripple, 4=rainbow
///    - R,G,B: base color (used as the "selected" color)
///    - per-key RGB: 16 keys × 3 bytes = 48 bytes of individual key colors
///
/// The K884x LED protocol (single encoded mode+color byte) is silently
/// ignored by 8850 firmware.
pub struct Keyboard8850 {
    inner: Keyboard884x,
}

impl Keyboard for Keyboard8850 {
    fn bind_key(&self, layer: u8, key: Key, expansion: &Macro, output: &mut Vec<u8>) -> Result<()> {
        self.inner.bind_key(layer, key, expansion, output)
    }

    fn set_led(&mut self, args: &[String], output: &mut Vec<u8>) -> Result<()> {
        let led_args = LedArgs::try_parse_from(args)?;

        let layer = led_args.layer;
        ensure!(layer < 3, "Layer must be 0-2");

        let mode = led_args.mode.mode_byte();
        let color = led_args.mode.color();

        // Send INIT packet - required for the 8850 to accept LED configuration.
        // The firmware enters a configuration mode upon receiving this.
        send_message(output, &[0x03, 0xFB, 0xFB, 0xFB]);

        // Build LED packet with per-key RGB data.
        // Format: [0x03, 0xFE, 0xB0, layer, mode, R, G, B, key0_R, key0_G, key0_B, ...]
        let mut msg = vec![0x03, 0xFE, 0xB0, layer, mode, color.r, color.g, color.b];
        for _ in 0..NUM_KEY_SLOTS {
            msg.extend_from_slice(&[color.r, color.g, color.b]);
        }
        // Truncate to 64 bytes (send_message pads to 64)
        msg.truncate(64);
        send_message(output, &msg);

        Ok(())
    }

    fn preferred_endpoint() -> u8 where Self: Sized {
        0x04
    }
}

impl Keyboard8850 {
    pub fn new(buttons: u8, knobs: u8) -> Result<Self> {
        Ok(Self {
            inner: Keyboard884x::new(buttons, knobs)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard::assert_messages;

    #[test]
    fn test_led_static_red() {
        let mut kb = Keyboard8850::new(16, 3).unwrap();
        let mut output = Vec::new();
        kb.set_led(&["led".to_string(), "0".to_string(), "static red".to_string()], &mut output).unwrap();

        assert_eq!(output.len(), 2 * 64); // INIT + LED packet

        // Check INIT packet
        assert_eq!(output[0], 0x03);
        assert_eq!(output[1], 0xFB);
        assert_eq!(output[2], 0xFB);
        assert_eq!(output[3], 0xFB);

        // Check LED packet
        let led = &output[64..128];
        assert_eq!(led[0], 0x03);
        assert_eq!(led[1], 0xFE);
        assert_eq!(led[2], 0xB0);
        assert_eq!(led[3], 0x00); // layer 0
        assert_eq!(led[4], 0x01); // mode 1 (static)
        assert_eq!(led[5], 0xFF); // R
        assert_eq!(led[6], 0x00); // G
        assert_eq!(led[7], 0x00); // B
        // First per-key color
        assert_eq!(led[8], 0xFF);  // key0 R
        assert_eq!(led[9], 0x00);  // key0 G
        assert_eq!(led[10], 0x00); // key0 B
    }

    #[test]
    fn test_led_off() {
        let mut kb = Keyboard8850::new(16, 3).unwrap();
        let mut output = Vec::new();
        kb.set_led(&["led".to_string(), "0".to_string(), "off".to_string()], &mut output).unwrap();

        let led = &output[64..128];
        assert_eq!(led[3], 0x00); // layer 0
        assert_eq!(led[4], 0x00); // mode 0 (off)
    }

    #[test]
    fn test_led_rainbow() {
        let mut kb = Keyboard8850::new(16, 3).unwrap();
        let mut output = Vec::new();
        kb.set_led(&["led".to_string(), "1".to_string(), "rainbow".to_string()], &mut output).unwrap();

        let led = &output[64..128];
        assert_eq!(led[3], 0x01); // layer 1
        assert_eq!(led[4], 0x04); // mode 4 (rainbow)
    }

    #[test]
    fn parse_led_modes() {
        assert_eq!("off".parse(), Ok(LedMode::Off));
        assert_eq!("static red".parse(), Ok(LedMode::Static(Color { r: 255, g: 0, b: 0 })));
        assert_eq!("reactive blue".parse(), Ok(LedMode::Reactive(Color { r: 0, g: 0, b: 255 })));
        assert_eq!("ripple green".parse(), Ok(LedMode::Ripple(Color { r: 0, g: 255, b: 0 })));
        assert_eq!("rainbow".parse(), Ok(LedMode::Rainbow));
    }

    #[test]
    fn parse_hex_color() {
        assert_eq!(Color::from_str("#FF8000"), Ok(Color { r: 255, g: 128, b: 0 }));
        assert_eq!(Color::from_str("#000000"), Ok(Color { r: 0, g: 0, b: 0 }));
    }
}
