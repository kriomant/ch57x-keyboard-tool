use anyhow::{ensure, Result};
use log::{debug};
use num::ToPrimitive;
use rusb::{Context, DeviceHandle};

use crate::keyboard::{Accord, LedColor};

use super::{Key, Keyboard, Macro, MouseAction, MouseEvent};

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

                // Count only key parts when putting header length
                let key_count = presses.iter().filter(|p| matches!(p, super::KeyboardPart::Key(_))).count();

                // Use actual key count. Using 0 for single-key breaks cases with a leading delay.
                msg.push(key_count as u8);

                for part in presses.iter() {
                    match part {
                        super::KeyboardPart::Key(Accord { modifiers, code }) => {
                            msg.extend_from_slice(&[modifiers.as_u8(), code.map_or(0, |c| c.value())]);
                        }
                        super::KeyboardPart::Delay(_) => {
                            // Delay entries are not part of the header payload for key programming.
                        }
                    }
                }
            }
            Macro::Media(code) => {
                let [low, high] = (*code as u16).to_le_bytes();
                msg.extend_from_slice(&[0, low, high, 0, 0, 0, 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Click(buttons), _)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                // Python encoding: [modifier, button, x, y, wheel]
                msg.push(5);
                msg.extend_from_slice(&[0, buttons.as_u8(), 0, 0, 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelUp, modifier)) => {
                msg.push(5);
                msg.extend_from_slice(&[modifier.map_or(0, |m| m as u8), 0, 0, 0, 1]);
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelDown, modifier)) => {
                msg.push(5);
                msg.extend_from_slice(&[modifier.map_or(0, |m| m as u8), 0, 0, 0, 255]);
            }
            // ...existing code...
            Macro::Mouse(MouseEvent(MouseAction::Move { dx, dy }, modifier)) => {
                // Encode dx/dy as low bytes (two's complement via cast) into Python-style positions:
                // [modifier, button/flag, x, y, wheel]
                let x_b = ((*dx as i32) & 0xff) as u8;
                let y_b = ((*dy as i32) & 0xff) as u8;
                msg.push(5);
                msg.extend_from_slice(&[modifier.map_or(0, |m| m as u8), 0, x_b, y_b, 0]);
            }
        };

        // Send main programming message (keys/media/mouse)
        self.send(&msg)?;

        // If macro has a leading delay part (we validated earlier that any delay must be leading),
        // send a single delay message with the specified ms after programming the macro.
        if let Macro::Keyboard(parts) = expansion {
            if let Some(super::KeyboardPart::Delay(ms)) = parts.first() {
                if *ms > 6000 {
                    return Err(anyhow::anyhow!("delay value {ms}ms exceeds maximum supported 6000ms"));
                }
                let mut delay_msg = msg.clone();
                delay_msg[4] = 0x05;
                let [low, high] = ms.to_le_bytes();
                delay_msg[5] = low;
                delay_msg[6] = high;
                self.send(&delay_msg)?;
            }
        }

        // Finish key binding
        self.send(&[0x03, 0xaa, 0xaa, 0, 0, 0, 0, 0, 0])?;
        self.send(&[0x03, 0xfd, 0xfe, 0xff])?;
        self.send(&[0x03, 0xaa, 0xaa, 0, 0, 0, 0, 0, 0])?;

        Ok(())
    }

    fn program_led(&self, mode: u8, layer: u8, color: LedColor) -> Vec<u8> {
        let mut m_c = <LedColor as ToPrimitive>::to_u8(&color).unwrap();
        m_c |= mode;
        debug!("mode and code: 0x{m_c:02} layer: {layer}");
        let mut msg = vec![0x03, 0xfe, 0xb0, layer, 0x08];
        msg.extend_from_slice(&[0; 5]);
        msg.extend_from_slice(&[0x01, 0x00, m_c]);
        msg.extend_from_slice(&[0; 52]);
        msg
    }

    fn end_program(&self) -> Vec<u8> {
        let mut msg = vec![0x03, 0xfd, 0xfe, 0xff];
        msg.extend_from_slice(&[0; 61]);
        msg
    }

    fn set_led(&mut self, mode: u8, layer: u8, color: LedColor) -> Result<()> {
        self.send(&self.program_led(mode, layer, color))?;
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

impl Keyboard884x {
    pub fn new(handle: DeviceHandle<Context>, endpoint: u8) -> Result<Self> {
        let mut keyboard = Self { handle, endpoint };

        keyboard.send(&[])?;

        Ok(keyboard)
    }
}
