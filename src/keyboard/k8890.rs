use anyhow::{ensure, Result};

use super::{Key, Keyboard, Macro, MouseAction, MouseEvent, send_message};

pub struct Keyboard8890;

impl Keyboard for Keyboard8890 {
    fn bind_key(&self, layer: u8, key: Key, expansion: &Macro, output: &mut Vec<u8>) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        // Start key binding
        send_message(output, &[0x03, 0xfe, layer+1, 0x1, 0x1, 0, 0, 0, 0]);

        match expansion {
            Macro::Keyboard(presses) => {
                ensure!(presses.len() <= 5, "macro sequence is too long");
                // For whatever reason empty key is added before others.
                let iter = presses.iter().map(|accord| (accord.modifiers.as_u8(), accord.code.map_or(0, |c| c.value())));
                let (len, items) = (presses.len() as u8, Box::new(std::iter::once((0, 0)).chain(iter)));
                for (i, (modifiers, code)) in items.enumerate() {
                    send_message(output, &[
                        0x03,
                        key.to_key_id(12)?,
                        ((layer+1) << 4) | expansion.kind(),
                        len,
                        i as u8,
                        modifiers,
                        code,
                        0,
                        0,
                    ]);
                }
            }
            Macro::Media(code) => {
                let [low, high] = (*code as u16).to_le_bytes();
                send_message(output, &[0x03, key.to_key_id(12)?, ((layer+1) << 4) | 0x02, low, high, 0, 0, 0, 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Click(buttons), modifier)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                send_message(output, &[0x03, key.to_key_id(12)?, ((layer+1) << 4) | 0x03, buttons.as_u8(), 0, 0, 0, modifier.map_or(0, |m| m as u8), 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelUp, modifier)) => {
                send_message(output, &[0x03, key.to_key_id(12)?, ((layer+1) << 4) | 0x03, 0, 0, 0, 0x01, modifier.map_or(0, |m| m as u8), 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelDown, modifier)) => {
                send_message(output, &[0x03, key.to_key_id(12)?, ((layer+1) << 4) | 0x03, 0, 0, 0, 0xff, modifier.map_or(0, |m| m as u8), 0]);
            }
        };

        // Finish key binding
        send_message(output, &[0x03, 0xaa, 0xaa, 0, 0, 0, 0, 0, 0]);

        Ok(())
    }

    fn set_led(&self, n: u8, output: &mut Vec<u8>) -> Result<()> {
        send_message(output, &[0x03, 0xa1, 0x01, 0, 0, 0, 0, 0, 0]);
        send_message(output, &[0x03, 0xb0, 0x18, n, 0, 0, 0, 0, 0]);
        send_message(output, &[0x03, 0xaa, 0xa1, 0, 0, 0, 0, 0, 0]);
        Ok(())
    }

    fn preferred_endpoint() -> u8 {
        0x02
    }
}

impl Keyboard8890 {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard::{Accord, Key, Macro, Modifier, MouseAction, MouseButton, MouseEvent, WellKnownCode, assert_messages};
    use enumset::EnumSet;

    #[test]
    fn test_keyboard_macro_bytes() {
        let keyboard = Keyboard8890::new();
        let mut output = Vec::new();

        // Test simple key press (Ctrl + A key)
        let a_key = Macro::Keyboard(vec![Accord::new(Modifier::Ctrl, Some(WellKnownCode::A.into()))]);
        keyboard.bind_key(0, Key::Button(0), &a_key, &mut output).unwrap();

        assert_messages(&output, &[
            &[0x03, 0xfe, 0x01, 0x01, 0x01], // binding start
            &[0x03, 0x01, 0x11, 0x01], // empty key
            &[0x03, 0x01, 0x11, 0x01, 0x01, 0x01, 0x04], // key press (Ctrl+A)
            &[0x03, 0xaa, 0xaa], // binding finish
        ]);
    }

    #[test]
    fn test_media_macro_bytes() {
        let keyboard = Keyboard8890::new();
        let mut output = Vec::new();

        // Test media key (Volume Up)
        let vol_up = Macro::Media(crate::keyboard::MediaCode::VolumeUp);
        keyboard.bind_key(0, Key::Button(1), &vol_up, &mut output).unwrap();

        assert_messages(&output, &[
            &[0x03, 0xfe, 0x01, 0x01, 0x01], // binding start
            &[0x03, 0x02, 0x12, 0xe9, 0x00], // media key
            &[0x03, 0xaa, 0xaa], // binding finish
        ]);
    }

    #[test]
    fn test_mouse_macro_bytes() {
        let keyboard = Keyboard8890::new();
        let mut output = Vec::new();

        // Test mouse click (Left button)
        let mut buttons = EnumSet::new();
        buttons.insert(MouseButton::Left);
        let left_click = Macro::Mouse(MouseEvent(MouseAction::Click(buttons), None));
        keyboard.bind_key(0, Key::Button(2), &left_click, &mut output).unwrap();

        assert_messages(&output, &[
            &[0x03, 0xfe, 0x01, 0x01, 0x01], // binding start
            &[0x03, 0x03, 0x13, 0x01], // mouse click
            &[0x03, 0xaa, 0xaa], // binding finish
        ]);
    }
}
