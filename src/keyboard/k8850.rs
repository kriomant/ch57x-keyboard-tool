//! Driver for the `514c:8850` protocol variant.
//!
//! Several physically different macropads ship with the `514c:8850` USB ID, and
//! they do *not* all speak the same protocol. This module implements the variant
//! that uses command byte `0xfd` and encodes every macro argument as a run of
//! 3-byte groups, as reverse-engineered from usbmon captures of the vendor's
//! `MINI_KEYBOARD` tool in
//! <https://github.com/kriomant/ch57x-keyboard-tool/issues/153>.
//!
//! # Wire format
//!
//! Every message is a 64-byte report with report ID `0x03`:
//!
//! ```text
//! offset  0     1     2       3       4      5        6        7...
//!        0x03  0xfd  key_id  layer   kind  n_mods  n_groups  3-byte groups
//! ```
//!
//! `n_mods + n_groups` is the total number of 3-byte groups that follow at
//! offset 7. Each group carries its payload in its *third* byte.
//!
//! For keyboard macros (`kind` = 1) each group is
//! `[delay_hi, delay_lo, hid_code]`, with modifiers emitted as their own groups
//! using the marker codes `0xf1`..=`0xf8`. A modifier applies only to the first
//! non-modifier key that follows it.
//!
//! For mouse macros (`kind` = 3) there are exactly 5 groups in fixed order:
//! modifier, buttons, X movement, Y movement, wheel.
//!
//! # Why this is not `k884x`
//!
//! The `k884x` driver writes the same logical fields at different offsets: it
//! packs keyboard macros as 2-byte `[modifiers, code]` pairs, and places the
//! wheel delta at offset 15. On this firmware offset 15 is the *X movement*
//! byte, so a `wheelup` binding programmed by `k884x` makes the pointer drift
//! sideways instead of scrolling. Mouse clicks happen to land correctly in both
//! encodings, which makes the mismatch easy to misdiagnose.

use anyhow::{bail, ensure, Result};
use log::debug;

use crate::keyboard::{
    send_message, Accord, KeyboardEvent, MacroOptions, Modifier, MouseButton, MouseButtons,
    MouseModifier,
};

use super::{Key, Keyboard, Macro, MouseAction, MouseEvent};

/// Marker code the firmware uses to represent a held modifier inside a
/// key sequence.
fn modifier_code(modifier: &Modifier) -> u8 {
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

fn mouse_modifier_code(modifier: &MouseModifier) -> u8 {
    match modifier {
        MouseModifier::Ctrl => 0xf1,
        MouseModifier::Shift => 0xf2,
        MouseModifier::Alt => 0xf3,
    }
}

pub struct Keyboard8850 {
    buttons: u8,
    knobs: u8,
}

impl Keyboard for Keyboard8850 {
    fn bind_key(&self, layer: u8, key: Key, expansion: &Macro, output: &mut Vec<u8>) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        debug!("bind {} on layer {} to {}", key, layer, expansion);

        let mut msg = vec![
            0x03,
            0xfd,
            self.to_key_id(key)?,
            layer + 1,
            expansion.kind(),
        ];

        match expansion {
            Macro::Keyboard(KeyboardEvent(MacroOptions { delay, .. }, presses)) => {
                ensure!(*delay <= 6000, "maximum supported delay is 6000ms");
                let [delay_hi, delay_lo] = delay.to_be_bytes();

                let mut groups: Vec<[u8; 3]> = Vec::new();
                for Accord { modifiers, code } in presses.iter() {
                    // Modifiers occupy their own groups and bind to the next key.
                    for modifier in modifiers.iter() {
                        groups.push([0, 0, modifier_code(&modifier)]);
                    }
                    groups.push([delay_hi, delay_lo, code.map_or(0, |c| c.value())]);
                }

                // The firmware accepts 19 groups' worth of space but only
                // stores 18.
                ensure!(groups.len() <= 18, "macro sequence is too long");

                msg.extend_from_slice(&[0, groups.len() as u8]);
                for group in groups {
                    msg.extend_from_slice(&group);
                }
            }

            Macro::Media(code) => {
                // Consumer-control usage is 16 bits, split across the value
                // bytes of two consecutive groups.
                let [low, high] = (*code as u16).to_le_bytes();
                msg.extend_from_slice(&[0, 2, 0, 0, low, 0, 0, high]);
            }

            Macro::Mouse(MouseEvent(action, modifier)) => {
                // 1 modifier group + 4 data groups = 5 groups of 3 bytes.
                let mut groups = [0u8; 17];
                groups[0] = 1; // modifier group count
                groups[1] = 4; // remaining group count
                groups[4] = modifier.map_or(0, |m| mouse_modifier_code(&m)); // offset 9
                match action {
                    MouseAction::Click(buttons) => {
                        groups[7] = to_button_bitmap(buttons)?; // offset 12
                    }
                    MouseAction::Move(dx, dy) => {
                        groups[10] = *dx as u8; // offset 15
                        groups[13] = *dy as u8; // offset 18
                    }
                    MouseAction::Wheel(delta) => {
                        groups[16] = *delta as u8; // offset 21
                    }
                    MouseAction::Drag(..) => {
                        // The firmware clicks and moves, but releases the
                        // button before moving, so a drag never holds.
                        bail!("mouse drag is not supported by this keyboard");
                    }
                }
                msg.extend_from_slice(&groups);
            }
        };

        send_message(output, &msg);

        // Commit the binding. The vendor tool sends this once after the whole
        // batch, but the firmware also accepts it after each binding.
        send_message(output, &[0x03, 0xfd, 0xfe, 0xff]);

        Ok(())
    }

    fn set_led(&mut self, args: &[String], output: &mut Vec<u8>) -> Result<()> {
        // LED programming uses a separate command (0xfe 0xb0) that is not part
        // of the key-binding format that differs between these variants, so the
        // 884x sequence is reused here.
        let mut delegate = super::k884x::Keyboard884x::new(0, 0)?;
        delegate.set_led(args, output)
    }

    fn preferred_endpoint() -> u8 {
        0x04
    }
}

fn to_button_bitmap(buttons: &MouseButtons) -> Result<u8> {
    ensure!(!buttons.is_empty(), "buttons must be given for click macro");
    let mut bitmap = 0u8;
    for button in buttons.iter() {
        match button {
            MouseButton::Left => bitmap |= 1,
            MouseButton::Right => bitmap |= 2,
            MouseButton::Middle => bitmap |= 4,
        };
    }
    Ok(bitmap)
}

impl Keyboard8850 {
    pub fn new(buttons: u8, knobs: u8) -> Result<Self> {
        ensure!(buttons <= 16, "at most 16 buttons are supported");
        ensure!(knobs <= 3, "at most 3 knobs are supported");
        Ok(Self { buttons, knobs })
    }

    /// Map a logical key to the firmware's key slot.
    ///
    /// Buttons are numbered from 1. Knobs start immediately after the button
    /// block, and each knob occupies three consecutive slots in ccw / press / cw
    /// order.
    ///
    /// The firmware reserves a fixed block of button slots, so a pad with fewer
    /// physical buttons still has its knobs at the higher IDs. On a 12-button
    /// pad the block is 15 wide, which puts the first knob at 16 — confirmed by
    /// reading the factory default mapping, where buttons emit `a`..`l` and the
    /// knobs emit `p`,`q`,`r` / `s`,`t`,`u` / `v`,`w`,`x` (slots 16-24, leaving
    /// the 13-15 gap unused). A 16-button pad needs a 16-wide block instead.
    fn to_key_id(&self, key: Key) -> Result<u8> {
        let button_block = self.buttons.max(15);
        match key {
            Key::Button(n) if n >= self.buttons => Err(anyhow::anyhow!("invalid key index")),
            Key::Button(n) => Ok(n + 1),
            Key::Knob(n, _) if n >= self.knobs => Err(anyhow::anyhow!("invalid knob index")),
            Key::Knob(n, action) => Ok(button_block + 1 + 3 * n + (action as u8)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard::{
        assert_messages, Key, KnobAction, Macro, MacroOptions, MediaCode, Modifier, Modifiers,
        MouseAction, MouseButton, MouseEvent, MouseModifier, WellKnownCode,
    };

    /// Commit message emitted after every binding.
    const COMMIT: &[u8] = &[0x03, 0xfd, 0xfe, 0xff];

    fn keyboard() -> Keyboard8850 {
        Keyboard8850::new(12, 3).unwrap()
    }

    #[test]
    fn keyboard_macro_puts_modifier_in_its_own_group() {
        let mut output = Vec::new();
        let ctrl_a = Macro::Keyboard(KeyboardEvent(
            MacroOptions::default(),
            vec![Accord::new(Modifier::Ctrl, Some(WellKnownCode::A.into()))],
        ));
        keyboard()
            .bind_key(0, Key::Button(0), &ctrl_a, &mut output)
            .unwrap();

        assert_messages(
            &output,
            &[
                &[
                    0x03, // report ID
                    0xfd, // bind command
                    0x01, // key ID (button 0 + 1)
                    0x01, // layer 0 + 1
                    0x01, // keyboard macro
                    0x00, // modifier group count
                    0x02, // group count: one modifier, one key
                    0x00, 0x00, 0xf1, // group: left ctrl marker
                    0x00, 0x00, 0x04, // group: 'a'
                ],
                COMMIT,
            ],
        );
    }

    #[test]
    fn keyboard_macro_encodes_delay_big_endian() {
        let mut output = Vec::new();
        let delayed = Macro::Keyboard(KeyboardEvent(
            MacroOptions { delay: 250 },
            vec![Accord::new(Modifiers::empty(), Some(WellKnownCode::A.into()))],
        ));
        keyboard()
            .bind_key(0, Key::Button(0), &delayed, &mut output)
            .unwrap();

        assert_messages(
            &output,
            &[
                &[
                    0x03, 0xfd, 0x01, 0x01, 0x01, 0x00, 0x01, //
                    0x00, 0xfa, 0x04, // 250ms big-endian, then 'a'
                ],
                COMMIT,
            ],
        );
    }

    #[test]
    fn media_macro_splits_usage_across_two_groups() {
        let mut output = Vec::new();
        let vol_up = Macro::Media(MediaCode::VolumeUp);
        keyboard()
            .bind_key(2, Key::Knob(2, KnobAction::RotateCW), &vol_up, &mut output)
            .unwrap();

        // Verified against hardware: knob 3 clockwise on layer 3.
        assert_messages(
            &output,
            &[
                &[
                    0x03, 0xfd, //
                    0x18, // key ID 24 = third knob, clockwise
                    0x03, // layer 2 + 1
                    0x02, // media macro
                    0x00, 0x02, //
                    0x00, 0x00, 0xe9, // usage low byte
                    0x00, 0x00, 0x00, // usage high byte
                ],
                COMMIT,
            ],
        );
    }

    #[test]
    fn wheel_delta_goes_in_the_wheel_group_not_x_movement() {
        // This is the regression that makes the k884x driver unusable here:
        // it writes the delta at offset 15, which this firmware reads as X
        // movement, so the pointer drifts sideways instead of scrolling.
        let mut output = Vec::new();
        let wheel_up = Macro::Mouse(MouseEvent(MouseAction::Wheel(1), None));
        keyboard()
            .bind_key(0, Key::Knob(0, KnobAction::RotateCCW), &wheel_up, &mut output)
            .unwrap();

        assert_messages(
            &output,
            &[
                &[
                    0x03, 0xfd, //
                    0x10, // key ID 16 = first knob, counter-clockwise
                    0x01, 0x03, // mouse macro
                    0x01, 0x04, // one modifier group, four data groups
                    0x00, 0x00, 0x00, // modifier
                    0x00, 0x00, 0x00, // buttons
                    0x00, 0x00, 0x00, // X movement
                    0x00, 0x00, 0x00, // Y movement
                    0x00, 0x00, 0x01, // wheel, at offset 21
                ],
                COMMIT,
            ],
        );
    }

    #[test]
    fn wheel_accepts_a_modifier() {
        // Matches the reference capture of the vendor tool byte for byte.
        let mut output = Vec::new();
        let ctrl_wheel = Macro::Mouse(MouseEvent(
            MouseAction::Wheel(1),
            Some(MouseModifier::Ctrl),
        ));
        keyboard()
            .bind_key(0, Key::Knob(0, KnobAction::RotateCCW), &ctrl_wheel, &mut output)
            .unwrap();

        assert_messages(
            &output,
            &[
                &[
                    0x03, 0xfd, 0x10, 0x01, 0x03, 0x01, 0x04, //
                    0x00, 0x00, 0xf1, // left ctrl
                    0x00, 0x00, 0x00, //
                    0x00, 0x00, 0x00, //
                    0x00, 0x00, 0x00, //
                    0x00, 0x00, 0x01, // wheel
                ],
                COMMIT,
            ],
        );
    }

    #[test]
    fn click_buttons_are_a_bitmap() {
        let mut output = Vec::new();
        let middle = Macro::Mouse(MouseEvent(
            MouseAction::Click(MouseButton::Middle.into()),
            None,
        ));
        keyboard()
            .bind_key(0, Key::Knob(0, KnobAction::Press), &middle, &mut output)
            .unwrap();

        assert_messages(
            &output,
            &[
                &[
                    0x03, 0xfd, 0x11, 0x01, 0x03, 0x01, 0x04, //
                    0x00, 0x00, 0x00, // modifier
                    0x00, 0x00, 0x04, // middle button bit, at offset 12
                ],
                COMMIT,
            ],
        );
    }

    #[test]
    fn drag_is_rejected() {
        let mut output = Vec::new();
        let drag = Macro::Mouse(MouseEvent(
            MouseAction::Drag(MouseButton::Left.into(), 10, 0),
            None,
        ));
        assert!(keyboard()
            .bind_key(0, Key::Button(0), &drag, &mut output)
            .is_err());
    }

    #[test]
    fn knobs_start_after_a_15_slot_button_block_on_a_12_button_pad() {
        let k = keyboard();
        assert_eq!(k.to_key_id(Key::Button(0)).unwrap(), 1);
        assert_eq!(k.to_key_id(Key::Button(11)).unwrap(), 12);
        // Slots 13-15 are reserved for a fourth knob this pad does not have.
        assert_eq!(k.to_key_id(Key::Knob(0, KnobAction::RotateCCW)).unwrap(), 16);
        assert_eq!(k.to_key_id(Key::Knob(0, KnobAction::RotateCW)).unwrap(), 18);
        assert_eq!(k.to_key_id(Key::Knob(2, KnobAction::RotateCW)).unwrap(), 24);
    }

    #[test]
    fn a_16_button_pad_needs_a_wider_button_block() {
        let k = Keyboard8850::new(16, 3).unwrap();
        assert_eq!(k.to_key_id(Key::Button(15)).unwrap(), 16);
        assert_eq!(k.to_key_id(Key::Knob(0, KnobAction::RotateCCW)).unwrap(), 17);
    }

    #[test]
    fn out_of_range_keys_are_rejected() {
        let k = keyboard();
        assert!(k.to_key_id(Key::Button(12)).is_err());
        assert!(k.to_key_id(Key::Knob(3, KnobAction::Press)).is_err());
    }
}
