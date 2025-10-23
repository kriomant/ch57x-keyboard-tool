use anyhow::{anyhow, ensure, Result};
use log::{debug};
use rusb::{Context, DeviceHandle};

use crate::keyboard::LedColor;
use crate::consts;

use super::{Key, Keyboard, Macro, MouseAction, MouseEvent};

pub struct Keyboard8890 {
    handle: DeviceHandle<Context>,
    endpoint: u8,
    led_programmed: bool,
}

impl Keyboard for Keyboard8890 {
    fn bind_key(&mut self, layer: u8, key: Key, expansion: &Macro) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        debug!("bind {} on layer {} to {}", key, layer, expansion);

        // Start key binding
        self.send(&[0x03, 0xfe, layer + 1, 0x1, 0x1, 0, 0, 0, 0])?;

        match expansion {
            Macro::Keyboard(presses) => {
                ensure!(presses.len() <= 5, "macro sequence is too long");
                // k8890 does not support delay parts; reject if present.
                ensure!(
                    presses.iter().all(|p| matches!(p, super::KeyboardPart::Key(_))),
                    "delays are not supported for this keyboard model"
                );

                // For whatever reason an empty key is added before others.
                let iter = presses.iter().map(|part| match part {
                    super::KeyboardPart::Key(accord) => (accord.modifiers.as_u8(), accord.code.map_or(0, |c| c.value())),
                    _ => (0, 0),
                });
                let (len, items) = (presses.len() as u8, Box::new(std::iter::once((0, 0)).chain(iter)));
                for (i, (modifiers, code)) in items.enumerate() {
                    self.send(&[
                        0x03,
                        key.to_key_id(12)?,
                        ((layer + 1) << 4) | expansion.kind(),
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
                self.send(&[0x03, key.to_key_id(12)?, ((layer + 1) << 4) | 0x02, low, high, 0, 0, 0, 0])?;
            }
            Macro::Mouse(MouseEvent(MouseAction::Click(buttons), modifier)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                self.send(&[0x03, key.to_key_id(12)?, ((layer + 1) << 4) | 0x03, buttons.as_u8(), 0, 0, 0, modifier.map_or(0, |m| m as u8), 0])?;
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelUp, modifier)) => {
                self.send(&[0x03, key.to_key_id(12)?, ((layer + 1) << 4) | 0x03, 0, 0, 0, 0x01, modifier.map_or(0, |m| m as u8), 0])?;
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelDown, modifier)) => {
                self.send(&[0x03, key.to_key_id(12)?, ((layer + 1) << 4) | 0x03, 0, 0, 0, 0xff, modifier.map_or(0, |m| m as u8), 0])?;
            }
            Macro::Mouse(MouseEvent(MouseAction::Move { dx, dy }, modifier)) => {
                // Encode relative movement. Negative values are represented as two's complement low byte.
                let dx_b = ((*dx as i32) & 0xff) as u8;
                let dy_b = ((*dy as i32) & 0xff) as u8;
                // Note: device interprets the two bytes in order (y, x) for horizontal/vertical mapping.
                self.send(&[0x03, key.to_key_id(12)?, ((layer + 1) << 4) | 0x03, 0, dy_b, dx_b, 0, modifier.map_or(0, |m| m as u8), 0])?;
            }
        };

        // Finish key binding
        self.send(&[0x03, 0xaa, 0xaa, 0, 0, 0, 0, 0, 0])?;

        Ok(())
    }

    fn program_led(&self, mode: u8, _layer: u8, _color: LedColor) -> Vec<u8> {
        let mut msg = vec![0x03, 0xb0, 0x18, mode];
        let size = consts::PACKET_SIZE - msg.len();
        msg.extend_from_slice(&vec![0; size]);
        msg
    }

    fn end_program(&self) -> Vec<u8> {
        let last_byte = if self.led_programmed { 0xa1 } else { 0xaa };
        let mut msg = vec![0x03, 0xaa, last_byte];
        let size = consts::PACKET_SIZE - msg.len();
        msg.extend_from_slice(&vec![0; size]);
        msg
    }

    fn set_led(&mut self, mode: u8, layer: u8, _color: LedColor) -> Result<()> {
        if mode > 2 {
            return Err(anyhow!("macropad supports modes 0, 1, and 2 only"));
        }
        self.led_programmed = true;
        self.send(&self.begin_program(layer))?;
        self.send(&self.program_led(mode, layer, LedColor::Red))?;
        self.send(&self.end_program())?;
        Ok(())
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

impl Keyboard8890 {
    pub fn new(handle: DeviceHandle<Context>, endpoint: u8) -> Result<Self> {
        let mut keyboard = Self { handle, endpoint, led_programmed: false };

        keyboard.send(&[])?;

        Ok(keyboard)
    }

    pub fn begin_program(&self, layer: u8) -> Vec<u8> {
        let mut msg = vec![0x03, 0xa1, layer];
        let size = consts::PACKET_SIZE - msg.len();
        msg.extend_from_slice(&vec![0; size]);
        msg
    }
}
