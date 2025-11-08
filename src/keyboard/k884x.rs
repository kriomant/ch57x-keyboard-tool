use std::str::FromStr;

use anyhow::{ensure, Result};
use clap::Parser;
use nom::{IResult, branch::alt, bytes::complete::tag, character::complete::alpha1, combinator::{map, map_res, value}, sequence::preceded};
use strum_macros::EnumString;

use crate::{keyboard::{Accord, MouseEvent}, parse::from_str};

use super::{Key, Keyboard, Macro, MouseAction, send_message};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
#[strum(serialize_all = "lowercase")]
#[repr(u8)]
pub enum LedColor {
    Red = 1,
    Orange = 2,
    Yellow = 3,
    Green = 4,
    Cyan = 5,
    Blue = 6,
    Purple = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
#[strum(serialize_all = "lowercase")]
#[repr(u8)]
pub enum LedBacklightColor {
    White = 0,
    Red = 1,
    Orange = 2,
    Yellow = 3,
    Green = 4,
    Cyan = 5,
    Blue = 6,
    Purple = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedMode {
    /// LEDs off
    Off,
    /// Backlight always on with specified color
    Backlight(LedBacklightColor),
    /// No backlight, shock effect with specified color when key pressed
    Shock(LedColor),
    /// No backlight, shock2 effect with specified color when key pressed
    Shock2(LedColor),
    /// No backlight, light up key with specified color when pressed
    Press(LedColor),
}

impl FromStr for LedMode {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        crate::parse::from_str(led_mode, s)
    }
}

impl LedMode {
    fn code(&self) -> u8 {
        let (mode, color) = match *self {
            LedMode::Off => (0, 0),
            LedMode::Backlight(LedBacklightColor::White) => (5, 0),
            LedMode::Backlight(color) => (1, color as u8),
            LedMode::Shock(color) => (2, color as u8),
            LedMode::Shock2(color) => (3, color as u8),
            LedMode::Press(color) => (4, color as u8),
        };
        (color << 4) | mode
    }
}

fn led_backlight_color(s: &str) -> IResult<&str, LedBacklightColor> {
    map_res(alpha1, LedBacklightColor::from_str)(s)
}

fn led_color(s: &str) -> IResult<&str, LedColor> {
    map_res(alpha1, LedColor::from_str)(s)
}

fn led_mode(s: &str) -> IResult<&str, LedMode> {
    let mut mode = alt((
        value(LedMode::Off, tag("off")),
        map(preceded(tag("backlight "), led_backlight_color), LedMode::Backlight),
        map(preceded(tag("shock2 "), led_color), LedMode::Shock2),
        map(preceded(tag("shock "), led_color), LedMode::Shock),
        map(preceded(tag("press "), led_color), LedMode::Press),
    ));
    mode(s)
}

fn parse_led_mode(s: &str) -> Result<LedMode, String> {
    from_str(led_mode, s).map_err(|e| format!("Invalid LED mode: {:?}", e))
}

#[derive(Parser, Debug)]
struct LedArgs {
    /// Layer to set the LED (0-based)
    layer: u8,

    /// LED mode
    #[arg(value_parser=parse_led_mode)]
    mode: LedMode,
}

pub struct Keyboard884x {
    buttons: u8,
    knobs: u8,
}

impl Keyboard for Keyboard884x {
    fn bind_key(&self, layer: u8, key: Key, expansion: &Macro, output: &mut Vec<u8>) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        let mut msg = vec![
            0x03,
            0xfe,
            self.to_key_id(key)?,
            layer + 1,
            expansion.kind(),
            0,
            0,
            0,
            0,
            0,
        ];

        match expansion {
            Macro::Keyboard(presses) => {
                ensure!(presses.len() <= 18, "macro sequence is too long");

                // Allow single key modifier to be used in combo with other key(s)
                if presses.len() == 1 && presses[0].code.is_none(){
                    msg.push(0);
                } else {
                    msg.push(presses.len() as u8);
                }

                for Accord { modifiers, code } in presses.iter() {
                    msg.extend_from_slice(&[modifiers.as_u8(), code.map_or(0, |c| c.value())]);
                }
            }
            Macro::Media(code) => {
                let [low, high] = (*code as u16).to_le_bytes();
                msg.extend_from_slice(&[0, low, high, 0, 0, 0, 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Move(dx, dy), modifier)) => {
                msg.extend_from_slice(&[0x05, modifier.map_or(0, |m| m as u8), 0, *dx as u8, *dy as u8]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Drag(buttons, dx, dy), modifier)) => {
                msg.extend_from_slice(&[0x05, modifier.map_or(0, |m| m as u8), buttons.as_u8(), *dx as u8, *dy as u8]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Click(buttons), modifier)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                msg.extend_from_slice(&[0x01, modifier.map_or(0, |m| m as u8), buttons.as_u8()]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Wheel(delta), modifier)) => {
                msg.extend_from_slice(&[0x03, modifier.map_or(0, |m| m as u8), 0, 0, 0, *delta as u8]);
            }
        };

        send_message(output, &msg);

        // Finish key binding
        send_message(output, &[0x03, 0xfd, 0xfe, 0xff]);

        Ok(())
    }

    fn set_led(&mut self, args: &[String], output: &mut Vec<u8>) -> Result<()> {
        let led_args = LedArgs::try_parse_from(
            std::iter::once("led".to_string()).chain(args.iter().cloned())
        )?;

        let layer = led_args.layer;
        ensure!(layer < 3, "Layer must be 0-2");

        let code = led_args.mode.code();

        // Program LED settings
        send_message(output, &[0x03, 0xfe, 0xb0, layer+1, 0x08, 0x00, 0x05, 0x01, 0x00, code, 0x00, 0x34]);

        // End programming sequence
        send_message(output, &[0x03, 0xfd, 0xfe, 0xff, 0x00, 0x3d]);

        Ok(())
    }

    fn preferred_endpoint() -> u8 {
        0x04
    }
}

impl Keyboard884x {
    pub fn new(buttons: u8, knobs: u8) -> Result<Self> {
        ensure!(
            (buttons <= 15 && knobs <= 3) ||
            (buttons <= 12 && knobs <= 4),
            "unsupported combination of buttons and knobs count"
        );
        Ok(Self { buttons, knobs })
    }

    fn to_key_id(&self, key: Key) -> Result<u8> {
        const MAX_NUMBER_OF_BUTTONS: u8 = 15;
        match key {
            Key::Button(n) if n >= MAX_NUMBER_OF_BUTTONS => Err(anyhow::anyhow!("invalid key index")),

            // There are keyboards with 15 buttons and 3 knobs, so NUMBER_OF_BUTTONS is correct
            // overall. However, there are keyboards with 12 buttons and 4 knobs, and fourth knob
            // doesn't use 25-27 codes as it should, but use 13-15, which are allocated for buttons.
            // So, it seems, they exchange one row of buttons to extra knob.
            Key::Button(n) if n >= 12 && self.knobs == 4 => Err(anyhow::anyhow!("invalid key index")),
            Key::Knob(4, action) if self.buttons <= 12 => Ok(13 + (action as u8)),

            Key::Button(n) => Ok(n + 1),
            Key::Knob(n, _) if n >= 3 => Err(anyhow::anyhow!("invalid knob index")),
            Key::Knob(n, action) => Ok(MAX_NUMBER_OF_BUTTONS + 1 + 3 * n + (action as u8)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard::{Key, KnobAction, Macro, Modifier, MouseAction, MouseButton, MouseEvent, WellKnownCode, assert_messages};
    use enumset::EnumSet;

    #[test]
    fn test_keyboard_macro_bytes() {
        let keyboard = Keyboard884x::new(12, 3).unwrap();
        let mut output = Vec::new();

        // Test simple key press (Ctrl + A key)
        let a_key = Macro::Keyboard(vec![Accord::new(Modifier::Ctrl, Some(WellKnownCode::A.into()))]);
        keyboard.bind_key(0, Key::Button(0), &a_key, &mut output).unwrap();

        assert_messages(&output, &[
            &[
                0x03, // Message header
                0xfe, // Bind command
                0x01, // Key ID (button 0 + 1)
                0x01, // Layer 0 + 1
                0x01, // Keyboard macro type
                0x00, 0x00, 0x00, 0x00, 0x00,
                0x01, // Single press
                0x01, // Ctrl modifier
                0x04, // A key code
            ],
            &[0x03, 0xfd, 0xfe, 0xff],
        ]);
    }

    #[test]
    fn test_media_macro_bytes() {
        let keyboard = Keyboard884x::new(12, 3).unwrap();
        let mut output = Vec::new();

        // Test media key (Volume Up)
        let vol_up = Macro::Media(crate::keyboard::MediaCode::VolumeUp);
        keyboard.bind_key(0, Key::Button(1), &vol_up, &mut output).unwrap();

        assert_messages(&output, &[
            &[
                0x03, // Message header
                0xfe, // Bind command
                0x02, // Key ID (button 1 + 1)
                0x01,
                0x02, // Media macro type
                0x00, // Empty count
                0x00, 0x00, 0x00, 0x00, 0x00,
                0xe9, // Volume Up code (low byte)
                0x00, // Volume Up code (high byte)
            ],
            &[0x03, 0xfd, 0xfe, 0xff],
        ]);
    }

    #[test]
    fn test_mouse_macro_bytes() {
        let keyboard = Keyboard884x::new(12, 3).unwrap();
        let mut output = Vec::new();

        // Test mouse click (Left button)
        let mut buttons = EnumSet::new();
        buttons.insert(MouseButton::Left);
        let left_click = Macro::Mouse(MouseEvent(MouseAction::Click(buttons), None));
        keyboard.bind_key(0, Key::Button(2), &left_click, &mut output).unwrap();

        assert_messages(&output, &[
            &[
                0x03, // Message header
                0xfe, // Bind command
                0x03, // Key ID (button 2 + 1)
                0x01, // Mouse action type (click)
                0x03, // Mouse macro type
                0x00, 0x00, 0x00, 0x00, 0x00,
                0x01, // Left button pressed
                0x00, 0x01,
            ],
            &[0x03, 0xfd, 0xfe, 0xff],
        ]);
    }

    #[test]
    fn test_mouse_move_bytes() {
        let keyboard = Keyboard884x::new(12, 3).unwrap();
        let mut output = Vec::new();

        // Test mouse move (dx=10, dy=-5)
        let mouse_move = Macro::Mouse(MouseEvent(MouseAction::Move(10, -5), None));
        keyboard.bind_key(0, Key::Button(3), &mouse_move, &mut output).unwrap();

        assert_messages(&output, &[
            &[
                0x03, // Message header
                0xfe, // Bind command
                0x04, // Key ID (button 3 + 1)
                0x01, // Layer 0 + 1
                0x03, // Mouse macro type
                0x00, 0x00, 0x00, 0x00, 0x00,
                0x05, // Move action type
                0x00, // No modifier
                0x00, // No buttons
                0x0a, // dx=10
                0xfb, // dy=-5 (as 251)
            ],
            &[0x03, 0xfd, 0xfe, 0xff],
        ]);
    }

    #[test]
    fn test_mouse_wheel_bytes() {
        let keyboard = Keyboard884x::new(12, 3).unwrap();
        let mut output = Vec::new();

        // Test mouse wheel (delta=3)
        let mouse_wheel = Macro::Mouse(MouseEvent(MouseAction::Wheel(3), None));
        keyboard.bind_key(0, Key::Button(4), &mouse_wheel, &mut output).unwrap();

        assert_messages(&output, &[
            &[
                0x03, // Message header
                0xfe, // Bind command
                0x05, // Key ID (button 4 + 1)
                0x01, // Layer 0 + 1
                0x03, // Mouse macro type
                0x00, 0x00, 0x00, 0x00, 0x00,
                0x03, // Wheel action type
                0x00, // No modifier
                0x00, 0x00, 0x00,
                0x03, // delta=3
            ],
            &[0x03, 0xfd, 0xfe, 0xff],
        ]);
    }

    #[test]
    fn test_mouse_drag_bytes() {
        let keyboard = Keyboard884x::new(12, 3).unwrap();
        let mut output = Vec::new();

        // Test mouse drag (Left button, dx=5, dy=10)
        let mut buttons = EnumSet::new();
        buttons.insert(MouseButton::Left);
        let mouse_drag = Macro::Mouse(MouseEvent(MouseAction::Drag(buttons, 5, 10), None));
        keyboard.bind_key(0, Key::Button(5), &mouse_drag, &mut output).unwrap();

        assert_messages(&output, &[
            &[
                0x03, // Message header
                0xfe, // Bind command
                0x06, // Key ID (button 5 + 1)
                0x01, // Layer 0 + 1
                0x03, // Mouse macro type
                0x00, 0x00, 0x00, 0x00, 0x00,
                0x05, // Drag action type
                0x00, // No modifier
                0x01, // Left button
                0x05, // dx=5
                0x0a, // dy=10
            ],
            &[0x03, 0xfd, 0xfe, 0xff],
        ]);
    }

    #[test]
    #[should_panic(expected="unsupported combination of buttons and knobs count")]
    fn test_keyboard_with_15_buttons_cant_have_fourth_knob() {
        Keyboard884x::new(15, 4).unwrap();
    }

    #[test]
    fn test_keyboard_with_12_buttons_can_have_fourth_knob() {
        let keyboard = Keyboard884x::new(12, 4).unwrap();
        let mut output = Vec::new();

        // Test mouse click (Left button)
        let mut buttons = EnumSet::new();
        buttons.insert(MouseButton::Left);
        let left_click = Macro::Mouse(MouseEvent(MouseAction::Click(buttons), None));
        keyboard.bind_key(0, Key::Knob(4, KnobAction::Press), &left_click, &mut output).unwrap();

        assert_messages(&output, &[
            &[
                0x03,
                0xfe,
                0x0e, // Fouth knob uses codes usually used by buttons 13-15
                0x01,
                0x03,
                0x00, 0x00, 0x00, 0x00, 0x00,
                0x01,
                0x00, 0x01,
            ],
            &[0x03, 0xfd, 0xfe, 0xff],
        ]);
    }

    #[test]
    fn parse_led_mode() {
        assert_eq!("off".parse(), Ok(LedMode::Off));
        assert_eq!("backlight white".parse(), Ok(LedMode::Backlight(LedBacklightColor::White)));
        assert_eq!("backlight red".parse(), Ok(LedMode::Backlight(LedBacklightColor::Red)));
        assert_eq!("shock purple".parse(), Ok(LedMode::Shock(LedColor::Purple)));
        assert_eq!("shock2 yellow".parse(), Ok(LedMode::Shock2(LedColor::Yellow)));
        assert_eq!("press green".parse(), Ok(LedMode::Press(LedColor::Green)));

        assert!("press black".parse::<LedMode>().is_err());
        assert!("boom red".parse::<LedMode>().is_err());
    }

    #[test]
    fn test_led_mode_code_encoding() {
        assert_eq!(LedMode::Off.code(), 0x00);
        assert_eq!(LedMode::Backlight(LedBacklightColor::White).code(), 0x05);
        assert_eq!(LedMode::Backlight(LedBacklightColor::Red).code(), 0x11);
        assert_eq!(LedMode::Backlight(LedBacklightColor::Blue).code(), 0x61);
        assert_eq!(LedMode::Shock(LedColor::Red).code(), 0x12);
        assert_eq!(LedMode::Shock2(LedColor::Green).code(), 0x43);
        assert_eq!(LedMode::Press(LedColor::Purple).code(), 0x74);
    }
}
