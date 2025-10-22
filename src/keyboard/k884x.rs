use anyhow::{bail, ensure, Result};

use crate::keyboard::Accord;

use super::{Key, Keyboard, Macro, MouseAction, MouseEvent, send_message};

pub struct Keyboard884x;

impl Keyboard for Keyboard884x {
    fn bind_key(&self, layer: u8, key: Key, expansion: &Macro, output: &mut Vec<u8>) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        let mut msg = vec![
            0x03,
            0xfe,
            key.to_key_id(15)?,
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
    pub fn new() -> Self {
        Self
    }
}
