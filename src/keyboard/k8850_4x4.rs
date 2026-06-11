use anyhow::{bail, ensure, Result};
use log::debug;

use crate::keyboard::{
    send_message, Accord, KeyboardEvent, Modifier, MouseButton, MouseButtons, MouseModifier,
};

use super::{Key, Keyboard, Macro, MouseAction, MouseEvent};

pub struct Keyboard8850_4x4;

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
    match modifier {
        MouseModifier::Ctrl => 0xf1,
        MouseModifier::Shift => 0xf2,
        MouseModifier::Alt => 0xf3,
    }
}

impl Keyboard for Keyboard8850_4x4 {
    fn bind_key(&self, layer: u8, key: Key, expansion: &Macro, output: &mut Vec<u8>) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        debug!("bind {} on layer {} to {}", key, layer, expansion);

        let mut msg = vec![
            0x03,
            0xfd,
            // key.to_key_id(16)?,
            self.to_key_id(key)?,
            layer + 1,
            expansion.kind(),
        ];

        match expansion {
            Macro::Keyboard(KeyboardEvent(_, presses)) => {
                let mut key_sequence = vec![];

                for Accord { modifiers, code } in presses.iter() {
                    for modifier in modifiers.iter() {
                        key_sequence.push([0u8, 0u8, get_modifier_code(&modifier)]);
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
            Macro::Mouse(MouseEvent(action, modifier)) => {
                let mut mouse_data = [1u8, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
                mouse_data[4] = modifier.map_or(0, |m| get_mouse_modifier_code(&m));

                match action {
                    MouseAction::Move(dx, dy) => {
                        mouse_data[10] = *dx as u8;
                        mouse_data[13] = *dy as u8;
                    }
                    MouseAction::Drag(_buttons, _dx, _dy) => {
                        bail!("Mouse Drag action is not supported");
                    }
                    MouseAction::Click(buttons) => {
                        mouse_data[7] = self.to_mouse_button(buttons)?;
                    }
                    MouseAction::Wheel(delta) => {
                        mouse_data[16] = *delta as u8;
                    }
                }

                msg.extend_from_slice(&mouse_data);
            }
        };

        send_message(output, &msg);

        // Finish key binding
        send_message(output, &[0x03, 0xfd, 0xfe, 0xff]);

        Ok(())
    }

    fn set_led(&mut self, _args: &[String], _output: &mut Vec<u8>) -> Result<()> {
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

impl Keyboard8850_4x4 {
    pub fn new() -> Self {
        Self
    }

    fn to_key_id(&self, key: Key) -> Result<u8> {
        const MAX_NUMBER_OF_BUTTONS: u8 = 16;
        match key {
            Key::Button(n) if n >= MAX_NUMBER_OF_BUTTONS => {
                Err(anyhow::anyhow!("invalid key index"))
            }
            Key::Button(n) => Ok(n + 1),
            Key::Knob(n, action) => Ok(MAX_NUMBER_OF_BUTTONS + 1 + 3 * n + (action as u8)),
        }
    }

    fn to_mouse_button(&self, buttons: &MouseButtons) -> Result<u8> {
        ensure!(!buttons.is_empty(), "buttons must be given for click macro");
        let mut button_bitmap = 0u8;
        for button in buttons.iter() {
            match button {
                MouseButton::Left => button_bitmap |= 1,
                MouseButton::Right => button_bitmap |= 2,
                MouseButton::Middle => button_bitmap |= 4,
            };
        }
        Ok(button_bitmap)
    }
}
