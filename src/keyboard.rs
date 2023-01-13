use crate::parse::{parse_accord, parse_macro};

use std::{time::Duration, str::FromStr, fmt::Display};

use log::debug;
use rusb::{DeviceHandle, GlobalContext};
use anyhow::{anyhow, ensure, Result};
use enumset::{EnumSetType, EnumSet};
use serde_with::DeserializeFromStr;
use strum_macros::{EnumString, Display};

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

        // Start key binding
        self.send([0xa1, layer+1, 0, 0, 0, 0, 0, 0])?;

        match expansion {
            Macro::Keyboard(presses) => {
                ensure!(presses.len() <= 5, "macro sequence is too long");
                // For whatever reason empty key is added before others.
                let iter = presses.iter().map(|accord| (accord.modifiers.as_u8(), accord.code as u8));
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
            Macro::Play => {} //(0, Box::new(std::iter::once((0, 0)))),

            Macro::Mouse(MouseEvent::Click(buttons)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                self.send([key.to_key_id()?, ((layer+1) << 4) | 0x03, buttons.as_u8(), 0, 0, 0, 0, 0])?;
            }
            Macro::Mouse(MouseEvent::WheelUp(modifier)) => {
                self.send([key.to_key_id()?, ((layer+1) << 4) | 0x03, 0, 0, 0, 0x01, modifier.map_or(0, |m| m as u8), 0])?;
            }
            Macro::Mouse(MouseEvent::WheelDown(modifier)) => {
                self.send([key.to_key_id()?, ((layer+1) << 4) | 0x03, 0, 0, 0, 0xff, modifier.map_or(0, |m| m as u8), 0])?;
            }
        };

        // Finish key binding
        self.send([0xaa, 0xaa, 0, 0, 0, 0, 0, 0])?;

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
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum KnobAction {
    RotateCCW,
    Press,
    RotateCW,
}

#[derive(Debug, Clone, Copy)]
pub enum Key {
    Button(u8),
    #[allow(unused)]
    Knob(u8, KnobAction),
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
    PrintScree,
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
    pub code: Code,
}

impl Accord {
    pub fn new<M>(modifiers: M, code: Code) -> Self
        where M: Into<Modifiers>
    {
        Self { modifiers: modifiers.into(), code }
    }
}

impl From<(Modifiers, Code)> for Accord {
    fn from((modifiers, code): (Modifiers, Code)) -> Self {
        Self { modifiers, code }
    }
}

impl FromStr for Accord {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        use nom::sequence::terminated;
        use nom::combinator::eof;
        use nom::Finish as _;
        match terminated(parse_accord, eof)(s).finish() {
            Ok((_, accord)) => Ok(accord),
            Err(nom::error::Error { input, code }) =>
                Err(nom::error::Error { input: input.to_owned(), code }),
        }
    }
}

impl Display for Accord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for m in self.modifiers {
            write!(f, "{}-", m)?;
        }
        write!(f, "{}", self.code)?;
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

#[derive(Debug, EnumSetType)]
pub enum MouseButton {
    Left, Right, Middle
}

pub type MouseButtons = EnumSet<MouseButton>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEvent {
    Click(MouseButtons),
    WheelUp(Option<MouseModifier>),
    WheelDown(Option<MouseModifier>),
}

#[derive(Debug, Clone, PartialEq, Eq, DeserializeFromStr)]
pub enum Macro {
    Keyboard(Vec<Accord>),
    #[allow(unused)]
    Play,
    #[allow(unused)]
    Mouse(MouseEvent),
}

impl Macro {
    fn kind(&self) -> u8 {
        match self {
            Macro::Keyboard(_) => 1,
            Macro::Play => 2,
            Macro::Mouse(_) => 3,
        }
    }
}

impl FromStr for Macro {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        use nom::sequence::terminated;
        use nom::combinator::eof;
        use nom::Finish as _;
        match terminated(parse_macro, eof)(s).finish() {
            Ok((_, accord)) => Ok(accord),
            Err(nom::error::Error { input, code }) =>
                Err(nom::error::Error { input: input.to_owned(), code }),
        }
    }
}
