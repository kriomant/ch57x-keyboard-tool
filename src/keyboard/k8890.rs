use anyhow::{ensure, Result};
use clap::Parser;

use crate::keyboard::MouseEvent;
use super::{Key, Keyboard, Macro, MouseAction, send_message};

pub struct Keyboard8890;

#[derive(Parser, Debug)]
struct LedArgs {
    /// LED mode
    mode: u8,
}

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
                        self.to_key_id(key)?,
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
                send_message(output, &[0x03, self.to_key_id(key)?, ((layer+1) << 4) | 0x02, low, high, 0, 0, 0, 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Move(dx, dy), modifier)) => {
                send_message(output, &[0x03, self.to_key_id(key)?, ((layer+1) << 4) | 0x03, 0, *dx as u8, *dy as u8, 0, modifier.map_or(0, |m| m as u8), 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Drag(buttons, dx, dy), modifier)) => {
                send_message(output, &[0x03, self.to_key_id(key)?, ((layer+1) << 4) | 0x03, buttons.as_u8(), *dx as u8, *dy as u8, 0, modifier.map_or(0, |m| m as u8), 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Click(buttons), modifier)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                send_message(output, &[0x03, self.to_key_id(key)?, ((layer+1) << 4) | 0x03, buttons.as_u8(), 0, 0, 0, modifier.map_or(0, |m| m as u8), 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Scroll(delta), modifier)) => {
                send_message(output, &[0x03, self.to_key_id(key)?, ((layer+1) << 4) | 0x03, 0, 0, 0, *delta as u8, modifier.map_or(0, |m| m as u8), 0]);
            }
        };

        // Finish key binding
        send_message(output, &[0x03, 0xaa, 0xaa, 0, 0, 0, 0, 0, 0]);

        Ok(())
    }

    fn set_led(&mut self, args: &[String], output: &mut Vec<u8>) -> Result<()> {
        let args = LedArgs::try_parse_from(args)?;
        send_message(output, &[0x03, 0xa1, 0x01, 0, 0, 0, 0, 0, 0]);
        send_message(output, &[0x03, 0xb0, 0x18, args.mode, 0, 0, 0, 0, 0]);
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

    fn to_key_id(&self, key: Key) -> Result<u8> {
        const BASE: u8 = 12;
        match key {
            Key::Button(n) if n >= BASE => Err(anyhow::anyhow!("invalid key index")),
            Key::Button(n) => Ok(n + 1),
            Key::Knob(n, _) if n >= 3 => Err(anyhow::anyhow!("invalid knob index")),
            Key::Knob(n, action) => Ok(BASE + 1 + 3 * n + (action as u8)),
        }
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

    #[test]
    fn test_mouse_move_bytes() {
        let keyboard = Keyboard8890::new();
        let mut output = Vec::new();

        // Test mouse move (dx=10, dy=-5)
        let mouse_move = Macro::Mouse(MouseEvent(MouseAction::Move(10, -5), None));
        keyboard.bind_key(0, Key::Button(3), &mouse_move, &mut output).unwrap();

        assert_messages(&output, &[
            &[0x03, 0xfe, 0x01, 0x01, 0x01], // binding start
            &[0x03, 0x04, 0x13, 0x00, 0x0a, 0xfb, 0x00, 0x00], // mouse move (dx=10, dy=-5 as 251)
            &[0x03, 0xaa, 0xaa], // binding finish
        ]);
    }

    #[test]
    fn test_mouse_scroll_bytes() {
        let keyboard = Keyboard8890::new();
        let mut output = Vec::new();

        // Test mouse scroll (delta=3)
        let mouse_scroll = Macro::Mouse(MouseEvent(MouseAction::Scroll(3), None));
        keyboard.bind_key(0, Key::Button(4), &mouse_scroll, &mut output).unwrap();

        assert_messages(&output, &[
            &[0x03, 0xfe, 0x01, 0x01, 0x01], // binding start
            &[0x03, 0x05, 0x13, 0x00, 0x00, 0x00, 0x03, 0x00], // mouse scroll (delta=3)
            &[0x03, 0xaa, 0xaa], // binding finish
        ]);
    }

    #[test]
    fn test_mouse_drag_bytes() {
        let keyboard = Keyboard8890::new();
        let mut output = Vec::new();

        // Test mouse drag (Left button, dx=5, dy=10)
        let mut buttons = EnumSet::new();
        buttons.insert(MouseButton::Left);
        let mouse_drag = Macro::Mouse(MouseEvent(MouseAction::Drag(buttons, 5, 10), None));
        keyboard.bind_key(0, Key::Button(5), &mouse_drag, &mut output).unwrap();

        assert_messages(&output, &[
            &[0x03, 0xfe, 0x01, 0x01, 0x01], // binding start
            &[0x03, 0x06, 0x13, 0x01, 0x05, 0x0a, 0x00, 0x00], // mouse drag (buttons=1, dx=5, dy=10)
            &[0x03, 0xaa, 0xaa], // binding finish
        ]);
    }
}
