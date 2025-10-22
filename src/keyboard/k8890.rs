use anyhow::{ensure, Result};

use crate::keyboard::MouseEvent;
use super::{Key, Keyboard, Macro, MouseAction, send_message};

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
            Macro::Mouse(MouseEvent(MouseAction::Move(dx, dy), modifier)) => {
                send_message(output, &[0x03, key.to_key_id(12)?, ((layer+1) << 4) | 0x03, 0, *dx as u8, *dy as u8, 0, modifier.map_or(0, |m| m as u8), 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Drag(buttons, dx, dy), modifier)) => {
                send_message(output, &[0x03, key.to_key_id(12)?, ((layer+1) << 4) | 0x03, buttons.as_u8(), *dx as u8, *dy as u8, 0, modifier.map_or(0, |m| m as u8), 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Click(buttons), modifier)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                send_message(output, &[0x03, key.to_key_id(12)?, ((layer+1) << 4) | 0x03, buttons.as_u8(), 0, 0, 0, modifier.map_or(0, |m| m as u8), 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Scroll(delta), modifier)) => {
                send_message(output, &[0x03, key.to_key_id(12)?, ((layer+1) << 4) | 0x03, 0, 0, 0, *delta as u8, modifier.map_or(0, |m| m as u8), 0]);
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
