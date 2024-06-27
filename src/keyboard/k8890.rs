use anyhow::{ensure, Result};
use log::debug;
use rusb::{Context, DeviceHandle};

use super::{Key, Keyboard, Macro, MouseAction, MouseEvent};

pub struct Keyboard8890 {
    handle: DeviceHandle<Context>,
    endpoint: u8,
}

impl Keyboard for Keyboard8890 {
    fn bind_key(&mut self, layer: u8, key: Key, expansion: &Macro) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        debug!("bind {} on layer {} to {}", key, layer, expansion);

        // Start key binding
        self.send(&[0x03, 0xfe, layer+1, 0x1, 0x1, 0, 0, 0, 0])?;

        match expansion {
            Macro::Keyboard(presses) => {
                ensure!(presses.len() <= 5, "macro sequence is too long");
                // For whatever reason empty key is added before others.
                let iter = presses.iter().map(|accord| (accord.modifiers.as_u8(), accord.code.map_or(0, |c| c.value())));
                let (len, items) = (presses.len() as u8, Box::new(std::iter::once((0, 0)).chain(iter)));
                for (i, (modifiers, code)) in items.enumerate() {
                    self.send(&[0x03, 
                        key.to_key_id(12)?,
                        ((layer+1) << 4) | expansion.kind(),
                        len,
                        i as u8,
                        modifiers,
                        code,
                        0,
                        0,
                    ])?;
                }
            }
            Macro::Media(code) => {
                let [low, high] = (*code as u16).to_le_bytes();
                self.send(&[0x03, key.to_key_id(12)?, ((layer+1) << 4) | 0x02, low, high, 0, 0, 0, 0])?;
            }
            Macro::Mouse(MouseEvent(MouseAction::Click(buttons), modifier)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                self.send(&[0x03, key.to_key_id(12)?, ((layer+1) << 4) | 0x03, buttons.as_u8(), 0, 0, 0, modifier.map_or(0, |m| m as u8), 0])?;
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelUp, modifier)) => {
                self.send(&[0x03, key.to_key_id(12)?, ((layer+1) << 4) | 0x03, 0, 0, 0, 0x01, modifier.map_or(0, |m| m as u8), 0])?;
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelDown, modifier)) => {
                self.send(&[0x03, key.to_key_id(12)?, ((layer+1) << 4) | 0x03, 0, 0, 0, 0xff, modifier.map_or(0, |m| m as u8), 0])?;
            }
        };

        // Finish key binding
        self.send(&[0x03, 0xaa, 0xaa, 0, 0, 0, 0, 0, 0])?;

        Ok(())
    }

    fn set_led(&mut self, n: u8) -> Result<()> {
        self.send(&[0x03, 0xa1, 0x01, 0, 0, 0, 0, 0, 0])?;
        self.send(&[0x03, 0xb0, 0x18, n, 0, 0, 0, 0, 0])?;
        self.send(&[0x03, 0xaa, 0xa1, 0, 0, 0, 0, 0, 0])?;
        Ok(())
    }

    fn get_handle(&self) -> &DeviceHandle<Context> {
        &self.handle
    }

    fn get_endpoint(&self) -> u8 {
        self.endpoint
    }

    fn preferred_endpoint() -> u8 {
        0x02
    }
}

impl Keyboard8890 {
    pub fn new(handle: DeviceHandle<Context>, endpoint: u8) -> Result<Self> {
        let mut keyboard = Self { handle, endpoint };

        keyboard.send(&[])?;

        Ok(keyboard)
    }
}
