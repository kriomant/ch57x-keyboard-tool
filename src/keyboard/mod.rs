pub(crate) mod k884x;
pub(crate) mod k8890;

use crate::parse;

use std::{time::Duration, str::FromStr, fmt::Display};
use num_derive::ToPrimitive;
use anyhow::{anyhow, ensure, Result};
use enumset::{EnumSetType, EnumSet};
use log::debug;
use rusb::{Context, DeviceHandle};
use serde_with::DeserializeFromStr;
use strum_macros::{EnumString, Display, EnumIter, EnumMessage};

use itertools::Itertools as _;

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(100);

pub trait Keyboard {
    fn bind_key(&mut self, layer: u8, key: Key, expansion: &Macro) -> Result<()>;
    fn set_led(&mut self, mode: u8, layer: u8, color: LedColor) -> Result<()>;
    fn program_led(&self, mode: u8, layer: u8, color: LedColor) -> Vec<u8>;
    fn end_program(&self) -> Vec<u8>;

    fn preferred_endpoint() -> u8 where Self: Sized;
    fn get_handle(&self) -> &DeviceHandle<Context>;
    fn get_endpoint(&self) -> u8;

    fn send(&mut self, msg: &[u8]) -> Result<()> {
        let mut buf = [0; 65];
        buf[..msg.len()].copy_from_slice(msg);

        debug!("send: {:02x?}", buf);
        let written = self
            .get_handle()
            .write_interrupt(self.get_endpoint(), &buf, DEFAULT_TIMEOUT)?;
        ensure!(written == buf.len(), "not all data written");
        Ok(())
    }
}

#[derive(Debug, Default, ToPrimitive, Clone, Copy, Display, clap::ValueEnum)]
pub enum LedColor {
    Red = 0x10,
    Orange = 0x20,
    Yellow = 0x30,
    Green = 0x40,
    #[default]
    Cyan = 0x50,
    Blue = 0x60,
    Purple = 0x70,
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
    fn to_key_id(self, base: u8) -> Result<u8> {
        match self {
            Key::Button(n) if n >= base => Err(anyhow!("invalid key index")),
            Key::Button(n) => Ok(n + 1),
            Key::Knob(n, _) if n >= 4 => Err(anyhow!("invalid knob index")),
            // Special case: 4th knob (index 3)
            Key::Knob(3, action) => Ok(13 + (action as u8)),
            // Default knob case
            Key::Knob(n, action) => Ok(base + 1 + 3 * n + (action as u8)),
        }
    }
}

#[derive(Debug, EnumSetType, EnumString, EnumIter, EnumMessage, Display)]
#[strum(ascii_case_insensitive)]
pub enum Modifier {
    #[strum(serialize="ctrl")]
    Ctrl,
    #[strum(serialize="shift")]
    Shift,
    #[strum(serialize="alt", serialize="opt")]
    Alt,
    #[strum(serialize="win", serialize="cmd")]
    Win,
    #[strum(serialize="rctrl")]
    RightCtrl,
    #[strum(serialize="rshift")]
    RightShift,
    #[strum(serialize="ralt", serialize="ropt")]
    RightAlt,
    #[strum(serialize="rwin", serialize="rcmd")]
    RightWin,
}

pub type Modifiers = EnumSet<Modifier>;


#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, EnumIter, EnumMessage, Display)]
#[repr(u16)]
#[strum(serialize_all="lowercase")]
#[strum(ascii_case_insensitive)]
pub enum MediaCode {
	Next = 0xb5,
    #[strum(serialize="previous", serialize="prev")]
	Previous = 0xb6,
	Stop = 0xb7,
	Play = 0xcd,
	Mute = 0xe2,
	VolumeUp = 0xe9,
	VolumeDown = 0xea,
	Favorites = 0x182,
	Calculator = 0x192,
	ScreenLock = 0x19e,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    WellKnown(WellKnownCode),
    Custom(u8),
}

impl Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Code::WellKnown(code) => write!(f, "{}", code),
            Code::Custom(code) => write!(f, "<{}>", code),
        }
    }
}

impl From<WellKnownCode> for Code {
    fn from(code: WellKnownCode) -> Self {
        Self::WellKnown(code)
    }
}

impl FromStr for Code {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parse::from_str(parse::code, s)
    }
}

impl Code {
    pub fn value(self) -> u8 {
        match self {
            Self::WellKnown(code) => code as u8,
            Self::Custom(code) => code,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, EnumIter, Display)]
#[repr(u8)]
#[strum(ascii_case_insensitive)]
#[strum(serialize_all="lowercase")]
pub enum WellKnownCode {
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
    #[strum(serialize="scrolllock", serialize="macbrightnessdown")]
    ScrollLock,
    #[strum(serialize="pause", serialize="macbrightnessup")]
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
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
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

#[derive(Debug, EnumSetType, EnumIter, Display)]
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
    /// Relative move in device units. Positive X = right, Positive Y = down.
    #[allow(dead_code)]
    Move { dx: i16, dy: i16 },
}

impl Display for MouseAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MouseAction::Click(buttons) => {
                write!(f, "{}", buttons.iter().format("+"))?;
            }
            MouseAction::WheelUp => { write!(f, "wheelup")?; }
            MouseAction::WheelDown => { write!(f, "wheeldown")?; }
            MouseAction::Move { dx, dy } => { write!(f, "move({},{})", dx, dy)?; }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyboardPart {
    Key(Accord),
    Delay(u16),
}

impl std::fmt::Display for KeyboardPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyboardPart::Key(accord) => write!(f, "{}", accord),
            KeyboardPart::Delay(ms) => write!(f, "delay[{}]", ms),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Macro {
    Keyboard(Vec<KeyboardPart>),
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

// Provide a custom Deserialize impl so we can return friendlier errors when
// users try to combine media tokens with delays/keys (media must be standalone).
impl<'de> serde::Deserialize<'de> for Macro {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize as string first
        let s = String::deserialize(deserializer)?;

        // If the user provided a comma-separated macro and any segment is a media
        // token, return a clear error explaining media macros must be standalone.
        if s.contains(',') {
            for seg in s.split(',') {
                let seg = seg.trim();
                if seg.parse::<MediaCode>().is_ok() {
                    return Err(serde::de::Error::custom(format!(
                        "media macros must be standalone: '{}' cannot be combined with delays or other keys",
                        seg
                    )));
                }
            }
        }

        // Fall back to existing FromStr parsing and surface its error if parsing fails.
        match s.parse::<Macro>() {
            Ok(m) => Ok(m),
            Err(e) => Err(serde::de::Error::custom(format!("invalid macro '{}': {}", s, e))),
        }
    }
}
