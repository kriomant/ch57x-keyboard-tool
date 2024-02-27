use std::time::Duration;

use anyhow::{ensure, Result};
use log::debug;
use rusb::{Context, DeviceHandle};

use super::{Key, Keyboard, Macro, MouseAction, MouseEvent, DEFAULT_TIMEOUT};

pub struct Keyboard8842 {
    handle: DeviceHandle<Context>,
    endpoint: u8,
}

impl Keyboard for Keyboard8842 {
    fn bind_key(&mut self, layer: u8, key: Key, expansion: &Macro) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        debug!("bind {} on layer {} to {}", key, layer, expansion);

        let mut msg = vec![0x03, 0xfe, key.to_key_id_16()?, layer+1, expansion.kind(), 0, 0, 0, 0, 0];

        match expansion {
            Macro::Keyboard(presses) => {
                ensure!(presses.len() <= 5, "macro sequence is too long");
                // For whatever reason empty key is added before others.
                let iter = presses.iter().map(|accord| (accord.modifiers.as_u8(), accord.code.map_or(0, |c| c.value())));

                msg.extend_from_slice(&[presses.len() as u8]);
                for (_i, (modifiers, code)) in iter.enumerate() {
                    msg.extend_from_slice(&[
                        modifiers,
                        code,
                    ]);
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


        let mut buf = [0; 65];
        buf.iter_mut().zip(msg.iter()).for_each(|(dst, src)| {
            *dst = *src;
        });
        self.send(&buf)?;

        Ok(())
    }

    fn set_led(&mut self, n: u8) -> Result<()> {
        todo!("LEDs");
        // self.send([0xa1, 0x01, 0, 0, 0, 0, 0, 0])?;
        // self.send([0xb0, 0x18, n, 0, 0, 0, 0, 0])?;
        // self.send([0xaa, 0xa1, 0, 0, 0, 0, 0, 0])?;
        Ok(())
    }
}

impl Keyboard8842 {
    pub fn new(handle: DeviceHandle<Context>, endpoint: u8) -> Result<Box<dyn Keyboard>> {
        let mut keyboard = Self { handle, endpoint };

        let mut buf = [0; 65];
        buf[0] = 0x03;
        keyboard.send(&buf)?;

        Ok(Box::new(keyboard))
    }

    fn send(&mut self, buf: &[u8]) -> Result<()> {
        debug!("send: {:02x?}", buf);
        let written = self.handle.write_interrupt(self.endpoint, &buf, DEFAULT_TIMEOUT)?;
        ensure!(written == buf.len(), "not all data written");
        Ok(())
    }
}

