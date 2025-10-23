use anyhow::{bail, ensure, Result};

use crate::keyboard::Accord;

use super::{Key, Keyboard, Macro, MouseAction, MouseEvent, send_message};

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
            Macro::Mouse(MouseEvent(MouseAction::Click(buttons), _)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                msg.extend_from_slice(&[0x01, 0, buttons.as_u8()]);
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelUp, modifier)) => {
                msg.extend_from_slice(&[0x03, modifier.map_or(0, |m| m as u8), 0, 0, 0, 0x1]);
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelDown, modifier)) => {
                msg.extend_from_slice(&[0x03, modifier.map_or(0, |m| m as u8), 0, 0, 0, 0xff]);
            }
        };

        send_message(output, &msg);

        // Finish key binding
        send_message(output, &[0x03, 0xfd, 0xfe, 0xff]);

        Ok(())
    }

    fn set_led(&self, _n: u8, _output: &mut Vec<u8>) -> Result<()> {
        bail!(
            "If you have a device which supports backlight LEDs, please let us know at \
               https://github.com/kriomant/ch57x-keyboard-tool/issues/60. We'll be glad to \
               help you reverse-engineer it."
        )
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
}
