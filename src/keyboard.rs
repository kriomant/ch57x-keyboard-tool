use std::time::Duration;

use log::debug;
use rusb::{DeviceHandle, GlobalContext};
use anyhow::{anyhow, ensure, Result};
use enumset::{EnumSetType, EnumSet};

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
        keyboard.send([0, 0, 0, 0, 0, 0])?;

        Ok(keyboard)
    }

    pub fn bind_key(&mut self, layer: u8, key: Key, expansion: &Macro) -> Result<()> {
        ensure!(layer < 3, "invalid layer index");

        // Start key binding
        self.send([0xa1, 0x01, 0, 0, 0, 0])?;

        let layer = 1; // 1..=3
        let (len, items): (u8, Box<dyn Iterator<Item=(u8, u8)>>) = match expansion {
            Macro::Keyboard(presses) => {
                ensure!(presses.len() <= 5, "macro sequence is too long");
                // For whatever reason empty key is added before others.
                let iter = presses.iter().map(|&(mods, code)| (mods.as_u8(), code as u8));
                (presses.len() as u8, Box::new(std::iter::once((0, 0)).chain(iter)))
            }
            Macro::Play => (0, Box::new(std::iter::once((0, 0)))),
            Macro::Mouse(s) => (0, Box::new(s.iter().map(|m| m.encode()))),
        };

        for (i, (modifiers, code)) in items.enumerate() {
            self.send([
                key.to_key_id()?,
                (layer << 4) | expansion.kind(),
                i as u8,
                len,
                modifiers,
                code,
            ])?;
        }

        // Finish key binding
        self.send([0xaa, 0xaa, 0, 0, 0, 0])?;

        Ok(())
    }

    fn send(&mut self, pkt: [u8; 6]) -> Result<()> {
        self.buf[1..7].copy_from_slice(pkt.as_slice());
        debug!("send: {:02x?}", self.buf);
        let written = self.handle.write_interrupt(self.endpoint, &self.buf, DEFAULT_TIMEOUT)?;
        ensure!(written == self.buf.len(), "not all data written");
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

#[derive(Debug, EnumSetType)]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
    Win,
    RCtrl,
    RShift,
    RAlt,
    RWin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(unused)]
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
    N1,
    N2,
    N3,
    N4,
    N5,
    N6,
    N7,
    N8,
    N9,
    N0,
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

#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {}

impl MouseEvent {
    fn encode(&self) -> (u8, u8) {
        (0, 0)
    }
}

#[derive(Debug, Clone)]
pub enum Macro {
    Keyboard(Vec<(EnumSet<Modifier>, Code)>),
    #[allow(unused)]
    Play,
    #[allow(unused)]
    Mouse(Vec<MouseEvent>),
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
