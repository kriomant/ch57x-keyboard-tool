use anyhow::{bail, ensure, Result};
use log::debug;
use rusb::{Context, DeviceHandle};

use crate::keyboard::{Accord, Modifier, MouseButton, MouseModifier};

use super::{Key, Keyboard, Macro, MouseAction, MouseEvent};

pub struct Keyboard8850_4x4 {
    handle: DeviceHandle<Context>,
    endpoint: u8,
}

fn get_modifier_code(modifier: &Modifier) -> u8 {
    match modifier {
        Modifier::Ctrl => 0xf1,
        Modifier::Shift => 0xf2,
        Modifier::Alt => 0xf3,
        Modifier::Win => 0xf4,
        Modifier::RightCtrl => 0xf5,
        Modifier::RightShift => 0xf6,
        Modifier::RightAlt => 0xf7,
        Modifier::RightWin => 0xf8,
    }
}

fn get_mouse_modifier_code(modifier: &MouseModifier) -> u8 {
    match modifier{
        MouseModifier::Ctrl => 0xf1,
        MouseModifier::Shift => 0xf2,
        MouseModifier::Alt => 0xf3,
    }
}

impl Keyboard for Keyboard8850_4x4 {
    fn bind_key(&mut self, layer: u8, key: Key, expansion: &Macro) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        debug!("bind {} on layer {} to {}", key, layer, expansion);

        let mut msg = vec![
            0x03,
            0xfd,
            key.to_key_id(16)?,
            layer + 1,
            expansion.kind(),
        ];

        match expansion {
            Macro::Keyboard(presses) => {
                let mut key_sequence = vec![];

                for Accord { modifiers, code } in presses.iter() {
                    for modifier in modifiers.iter() {
                        key_sequence.push(
                            [0u8, 0u8, get_modifier_code(&modifier)]
                        );
                    }
                    key_sequence.push([0u8, 0u8, code.map_or(0, |c| c.value())]);
                }

                ensure!(key_sequence.len() <= 18, "macro sequence is too long");

                msg.extend_from_slice(&[0, key_sequence.len() as u8]);

                for key in key_sequence {
                    msg.extend_from_slice(&key);
                }
            }
            Macro::Media(code) => {
                let [low, high] = (*code as u16).to_le_bytes();
                msg.extend_from_slice(&[0, 2, 0, 0, low, 0, 0, high]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Click(buttons), modifier)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                let mut button_bitmap = 0u8;
                for button in buttons.iter() {
                    match button {
                        MouseButton::Left => {button_bitmap |= 1}
                        MouseButton::Right => {button_bitmap |= 2}
                        MouseButton::Middle => {button_bitmap |= 4}
                    };
                }
                msg.extend_from_slice(&[1, 4, 0, 0, modifier.map_or(0, |m| get_mouse_modifier_code(&m)), 0, 0, button_bitmap]);
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelUp, modifier)) => {
                msg.extend_from_slice(&[1, 4, 0, 0, modifier.map_or(0, |m| get_mouse_modifier_code(&m)), 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x1]);
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelDown, modifier)) => {
                msg.extend_from_slice(&[1, 4, 0, 0, modifier.map_or(0, |m| get_mouse_modifier_code(&m)), 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff]);
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

    fn preferred_endpoint() -> u8 {
        0x04
    }

    fn get_handle(&self) -> &DeviceHandle<Context> {
        &self.handle
    }

    fn get_endpoint(&self) -> u8 {
        self.endpoint
    }

    fn get_payload_size(&self) -> usize { 65 }
}

impl Keyboard8850_4x4 {
    pub fn new(handle: DeviceHandle<Context>, endpoint: u8) -> Result<Self> {
        let mut keyboard = Self { handle, endpoint };

        keyboard.send(&[])?;

        Ok(keyboard)
    }
}
