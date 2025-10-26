use anyhow::{bail, ensure, Result};
use log::debug;
use rusb::{Context, DeviceHandle};

use crate::keyboard::{Accord, MouseEvent, ScrollDirection};

use super::{Key, Keyboard, Macro, MouseAction};

pub struct Keyboard884x {
    handle: DeviceHandle<Context>,
    endpoint: u8,
}

impl Keyboard for Keyboard884x {
    fn bind_key(&mut self, layer: u8, key: Key, expansion: &Macro) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        debug!("bind {} on layer {} to {}", key, layer, expansion);

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
            Macro::Mouse(MouseEvent(MouseAction::Scroll(ScrollDirection::Up), modifier)) => {
                msg.extend_from_slice(&[0x03, modifier.map_or(0, |m| m as u8), 0, 0, 0, 0x1]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Scroll(ScrollDirection::Down), modifier)) => {
                msg.extend_from_slice(&[0x03, modifier.map_or(0, |m| m as u8), 0, 0, 0, 0xff]);
            }
        };

        self.send(&msg)?;

        // Finish key binding
        self.send(&[0x03, 0xfd, 0xfe, 0xff])?;

        Ok(())
    }

    fn set_led(&mut self, _n: u8) -> Result<()> {
        bail!(
            "If you have a device which supports backlight LEDs, please let us know at \
               https://github.com/kriomant/ch57x-keyboard-tool/issues/60. We'll be glad to \
               help you reverse-engineer it."
        )
    }

    fn get_handle(&self) -> &DeviceHandle<Context> {
        &self.handle
    }

    fn get_endpoint(&self) -> u8 {
        self.endpoint
    }

    fn preferred_endpoint() -> u8 {
        0x04
    }
}

impl Keyboard884x {
    pub fn new(handle: DeviceHandle<Context>, endpoint: u8) -> Result<Self> {
        let mut keyboard = Self { handle, endpoint };

        keyboard.send(&[])?;

        Ok(keyboard)
    }
}
