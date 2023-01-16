use crate::parse;

use std::{time::Duration, str::FromStr, fmt::Display};

use log::debug;
use rusb::{DeviceHandle, GlobalContext};
use anyhow::{anyhow, ensure, Result};
use enumset::{EnumSetType, EnumSet};
use serde_with::DeserializeFromStr;
use strum_macros::{EnumString, Display};

use itertools::Itertools as _;

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(100);

pub struct Keyboard {
    handle: DeviceHandle<GlobalContext>,
    endpoint: u8,
    buf: [u8; 65],
}

impl Keyboard {
    pub fn new(handle: DeviceHandle<GlobalContext>, endpoint: u8) -> Result<Self> {
        let mut keyboard = Self { handle, endpoint, buf: [0; 65] };

        keyboard.buf[0] = 0x03;
        keyboard.send([0, 0, 0, 0, 0, 0, 0, 0])?;

        Ok(keyboard)
    }

    pub fn bind_key(&mut self, layer: u8, key: Key, expansion: &Macro) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        debug!("bind {} on layer {} to {}", key, layer, expansion);

        // Start key binding
        self.send([0xa1, layer+1, 0, 0, 0, 0, 0, 0])?;

        match expansion {
            Macro::Keyboard(presses) => {
                ensure!(presses.len() <= 5, "macro sequence is too long");
                // For whatever reason empty key is added before others.
                let iter = presses.iter().map(|accord| (accord.modifiers.as_u8(), accord.code.map_or(0, |c| c as u8)));
                let (len, items) = (presses.len() as u8, Box::new(std::iter::once((0, 0)).chain(iter)));
                for (i, (modifiers, code)) in items.enumerate() {
                    self.send([
                        key.to_key_id()?,
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
                self.send([key.to_key_id()?, ((layer+1) << 4) | 0x02, *code as u8, 0, 0, 0, 0, 0])?;
            }

            Macro::Mouse(MouseEvent(MouseAction::Click(buttons), modifier)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                self.send([key.to_key_id()?, ((layer+1) << 4) | 0x03, buttons.as_u8(), 0, 0, 0, modifier.map_or(0, |m| m as u8), 0])?;
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelUp, modifier)) => {
                self.send([key.to_key_id()?, ((layer+1) << 4) | 0x03, 0, 0, 0, 0x01, modifier.map_or(0, |m| m as u8), 0])?;
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelDown, modifier)) => {
                self.send([key.to_key_id()?, ((layer+1) << 4) | 0x03, 0, 0, 0, 0xff, modifier.map_or(0, |m| m as u8), 0])?;
            }
        };

        // Finish key binding
        self.send([0xaa, 0xaa, 0, 0, 0, 0, 0, 0])?;

        Ok(())
    }

    pub fn set_led(&mut self, n: u8) -> Result<()> {
        self.send([0xa1, 0x01, 0, 0, 0, 0, 0, 0])?;
        self.send([0xb0, 0x18, n, 0, 0, 0, 0, 0])?;
        self.send([0xaa, 0xa1, 0, 0, 0, 0, 0, 0])?;
        Ok(())
    }

    fn send(&mut self, pkt: [u8; 8]) -> Result<()> {
        self.buf[1..9].copy_from_slice(pkt.as_slice());
        debug!("send: {:02x?}", self.buf);
        let written = self.handle.write_interrupt(self.endpoint, &self.buf, DEFAULT_TIMEOUT)?;
        ensure!(written == self.buf.len(), "not all data written");
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, Display)]
#[repr(u8)]
pub enum KnobAction {
    #[strum(serialize="ccw")]
    RotateCCW,
    #[strum(serialize="press")]
    Press,
    #[strum(serialize="cw")]
    RotateCW,
}

#[derive(Debug, Clone, Copy)]
pub enum Key {
    Button(u8),
    #[allow(unused)]
    Knob(u8, KnobAction),
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Button(n) => write!(f, "button {}", n),
            Self::Knob(n, action) => write!(f, "knob {} {}", n, action),
        }
    }
}

impl Key {
    fn to_key_id(self) -> Result<u8> {
        match self {
            Key::Button(n) if n >= 12 => Err(anyhow!("invalid key index")),
            Key::Button(n) => Ok(n + 1),
            Key::Knob(n, _) if n >= 3 => Err(anyhow!("invalid knob index")),
            Key::Knob(n, action) => Ok(13 + 3*n + (action as u8)),
        }
    }
}

#[derive(Debug, EnumSetType, EnumString, Display)]
#[strum(ascii_case_insensitive)]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
    Win,
    #[strum(serialize="rctrl")]
    RightCtrl,
    #[strum(serialize="rshift")]
    RightShift,
    #[strum(serialize="ralt")]
    RightAlt,
    #[strum(serialize="rwin")]
    RightWin,
}

pub type Modifiers = EnumSet<Modifier>;


#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
#[repr(u8)]
#[strum(ascii_case_insensitive)]
pub enum MediaCode {
	Play = 0xcd,
    #[strum(serialize="previous", serialize="prev")]
	Previous = 0xb6,
	Next = 0xb5,
	Mute = 0xe2,
	VolumeUp = 0xe9,
	VolumeDown = 0xea,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
#[repr(u8)]
#[strum(ascii_case_insensitive)]
pub enum Code {
    A = 0x04,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    #[strum(serialize="1")] N1,
    #[strum(serialize="2")] N2,
    #[strum(serialize="3")] N3,
    #[strum(serialize="4")] N4,
    #[strum(serialize="5")] N5,
    #[strum(serialize="6")] N6,
    #[strum(serialize="7")] N7,
    #[strum(serialize="8")] N8,
    #[strum(serialize="9")] N9,
    #[strum(serialize="0")] N0,
    Enter,
    Escape,
    Backspace,
    Tab,
    Space,
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    Backslash,
    NonUSHash,
    Semicolon,
    Quote,
    Grave,
    Comma,
    Dot,
    Slash,
    CapsLock,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    PrintScreen,
    ScrollLock,
    Pause,
    Insert,
    Home,
    PageUp,
    Delete,
    End,
    PageDown,
    Right,
    Left,
    Down,
    Up,
    NumLock,
    NumPadSlash,
    NumPadAsterisk,
    NumPadMinus,
    NumPadPlus,
    NumPadEnter,
    NumPad1,
    NumPad2,
    NumPad3,
    NumPad4,
    NumPad5,
    NumPad6,
    NumPad7,
    NumPad8,
    NumPad9,
    NumPad0,
    NumPadDot,
    NonUSBackslash,
    Application,
    Power,
    NumPadEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, DeserializeFromStr)]
pub struct Accord {
    pub modifiers: Modifiers,
    pub code: Option<Code>,
}

impl Accord {
    pub fn new<M>(modifiers: M, code: Option<Code>) -> Self
        where M: Into<Modifiers>
    {
        Self { modifiers: modifiers.into(), code }
    }
}

impl From<(Modifiers, Option<Code>)> for Accord {
    fn from((modifiers, code): (Modifiers, Option<Code>)) -> Self {
        Self { modifiers, code }
    }
}

impl FromStr for Accord {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parse::from_str(parse::accord, s)
    }
}

impl Display for Accord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.modifiers.iter().format("-"))?;
        if let Some(code) = self.code {
            if !self.modifiers.is_empty() {
                write!(f, "-")?;
            }
            write!(f, "{}", code)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
#[strum(ascii_case_insensitive)]
#[repr(u8)]
pub enum MouseModifier {
    Ctrl = 0x01,
    Shift = 0x02,
    Alt = 0x04,
}

#[derive(Debug, EnumSetType, Display)]
pub enum MouseButton {
    #[strum(serialize="click")]
    Left,
    #[strum(serialize="rclick")]
    Right,
    #[strum(serialize="mclick")]
    Middle
}

pub type MouseButtons = EnumSet<MouseButton>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAction {
    Click(MouseButtons),
    WheelUp,
    WheelDown,
}

impl Display for MouseAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MouseAction::Click(buttons) => {
                write!(f, "{}", buttons.iter().format("+"))?;
            }
            MouseAction::WheelUp => { write!(f, "wheelup")?; }
            MouseAction::WheelDown => { write!(f, "wheeldown")?; }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent(pub MouseAction, pub Option<MouseModifier>);

impl Display for MouseEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(action, modifier) = self;
        if let Some(modifier) = modifier {
            write!(f, "{}-", modifier)?;
        }
        write!(f, "{}", action)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, DeserializeFromStr)]
pub enum Macro {
    Keyboard(Vec<Accord>),
    #[allow(unused)]
    Media(MediaCode),
    #[allow(unused)]
    Mouse(MouseEvent),
}

impl Macro {
    fn kind(&self) -> u8 {
        match self {
            Macro::Keyboard(_) => 1,
            Macro::Media(_) => 2,
            Macro::Mouse(_) => 3,
        }
    }
}

impl FromStr for Macro {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parse::from_str(parse::r#macro, s)
    }
}

impl Display for Macro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Macro::Keyboard(accords) => {
                write!(f, "{}", accords.iter().format(","))
            }
            Macro::Media(code) => {
                write!(f, "{}", code)
            }
            Macro::Mouse(event) => {
                write!(f, "{}", event)
            }
        }
    }
}
