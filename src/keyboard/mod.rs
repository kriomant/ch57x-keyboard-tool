pub(crate) mod k884x;
pub(crate) mod k8890;

use crate::parse;

use std::{time::Duration, str::FromStr, fmt::Display};

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
    fn set_led(&mut self, n: u8) -> Result<()>;

    fn preferred_endpoint() -> u8 where Self: Sized;
    fn get_handle(&self) -> &DeviceHandle<Context>;
    fn get_endpoint(&self) -> u8;

    fn send(&mut self, msg: &[u8]) -> Result<()> {
        let mut buf = [0; 64];
        buf[..msg.len()].copy_from_slice(msg);

        debug!("send: {:02x?}", buf);
        let written = self
            .get_handle()
            .write_interrupt(self.get_endpoint(), &buf, DEFAULT_TIMEOUT)?;
        ensure!(written == buf.len(), "not all data written");
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
    fn to_key_id(self, base: u8) -> Result<u8> {
        match self {
            Key::Button(n) if n >= base => Err(anyhow!("invalid key index")),
            Key::Button(n) => Ok(n + 1),
            Key::Knob(n, _) if n >= 3 => Err(anyhow!("invalid knob index")),
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

/* From section 12 at https://www.freebsddiary.org/APC/usb_hid_usages.php */
	ConsumerControl = 0x01,
	NumericKeyPad = 0x02,
	ProgrammableButtons = 0x03,
    #[strum(serialize="+10", serialize="P10")]
	P10 = 0x20,
    #[strum(serialize="+100", serialize="P100")]
	P100 = 0x21,
    #[strum(serialize="am/pm", serialize="am_pm")]
	AmPm = 0x22,
	Power = 0x30,
	Reset = 0x31,
	Sleep = 0x32,
	SleepAfter = 0x33,
	SleepMode = 0x34,
	Illumination = 0x35,
	FunctionButtons = 0x36,
	Menu = 0x40,
	MenuPick = 0x41,
	MenuUp = 0x42,
	MenuDown = 0x43,
	MenuLeft = 0x44,
	MenuRight = 0x45,
	MenuEscape = 0x46,
	MenuValueIncrease = 0x47,
	MenuValueDecrease = 0x48,
	DataOnScreen = 0x60,
	ClosedCaption = 0x61,
	ClosedCaptionSelect = 0x62,
    #[strum(serialize="vcr/tv", serialize="vcrTv")]
	VcrTv = 0x63,
	BroadcastMode = 0x64,
	Snapshot = 0x65,
	Still = 0x66,
	Selection = 0x80,
	AssignSelection = 0x81,
	ModeStep = 0x82,
	RecallLast = 0x83,
	EnterChannel = 0x84,
	OrderMovie = 0x85,
	Channel = 0x86,
	MediaSelection = 0x87,
	MediaSelectComputer = 0x88,
	MediaSelectTV = 0x89,
	MediaSelectWWW = 0x8A,
	MediaSelectDVD = 0x8B,
	MediaSelectTelephone = 0x8C,
	MediaSelectProgramGuide = 0x8D,
	MediaSelectVideoPhone = 0x8E,
	MediaSelectGames = 0x8F,
	MediaSelectMessages = 0x90,
	MediaSelectCD = 0x91,
	MediaSelectVCR = 0x92,
	MediaSelectTuner = 0x93,
	Quit = 0x94,
	Help = 0x95,
	MediaSelectTape = 0x96,
	MediaSelectCable = 0x97,
	MediaSelectSatellite = 0x98,
	MediaSelectSecurity = 0x99,
	MediaSelectHome = 0x9A,
	MediaSelectCall = 0x9B,
	ChannelIncrement = 0x9C,
	ChannelDecrement = 0x9D,
	MediaSelectSAP = 0x9E,
	VCRPlus = 0xA0,
	Once = 0xA1,
	Daily = 0xA2,
	Weekly = 0xA3,
	Monthly = 0xA4,
	Play = 0xB0,
	Pause = 0xB1,
	Record = 0xB2,
	FastForward = 0xB3,
	Rewind = 0xB4,
    #[strum(serialize="ScanNextTrack", serialize="Next")]
	ScanNextTrack = 0xB5,
    #[strum(serialize="ScanPreviousTrack", serialize="prev", serialize="previous")]
	ScanPreviousTrack = 0xB6,
	Stop = 0xB7,
	Eject = 0xB8,
	RandomPlay = 0xB9,
	SelectDisC = 0xBA,
	EnterDisc = 0xBB,
	Repeat = 0xBC,
	Tracking = 0xBD,
	TrackNormal = 0xBE,
	SlowTracking = 0xBF,
	FrameForward = 0xC0,
	FrameBack = 0xC1,
	Mark = 0xC2,
	ClearMark = 0xC3,
	RepeatFromMark = 0xC4,
	ReturnToMark = 0xC5,
	SearchMarkForward = 0xC6,
	SearchMarkBackwards = 0xC7,
	CounterReset = 0xC8,
	ShowCounter = 0xC9,
	TrackingIncrement = 0xCA,
	TrackingDecrement = 0xCB,
	Volume = 0xE0,
	Balance = 0xE1,
	Mute = 0xE2,
	Bass = 0xE3,
	Treble = 0xE4,
	BassBoost = 0xE5,
	SurroundMode = 0xE6,
	Loudness = 0xE7,
	MPX = 0xE8,
	VolumeUp = 0xE9,
	VolumeDown = 0xEA,
	SpeedSelect = 0xF0,
	PlaybackSpeed = 0xF1,
	StandardPlay = 0xF2,
	LongPlay = 0xF3,
	ExtendedPlay = 0xF4,
	Slow = 0xF5,
	FanEnable = 0x100,
	FanSpeed = 0x101,
	Light = 0x102,
	LightIlluminationLevel = 0x103,
	ClimateControlEnable = 0x104,
	RoomTemperature = 0x105,
	SecurityEnable = 0x106,
	FireAlarm = 0x107,
	PoliceAlarm = 0x108,
	BalanceRight = 0x150,
	BalanceLeft = 0x151,
	BassIncrement = 0x152,
	BassDecrement = 0x153,
	TrebleIncrement = 0x154,
	TrebleDecrement = 0x155,
	SpeakerSystem = 0x160,
	ChannelLeft = 0x161,
	ChannelRight = 0x162,
	ChannelCenter = 0x163,
	ChannelFront = 0x164,
	ChannelCenterFront = 0x165,
	ChannelSide = 0x166,
	ChannelSurround = 0x167,
	ChannelLowFrequencyEnhancement = 0x168,
	ChannelTop = 0x169,
	ChannelUnknown = 0x16A,
    #[strum(serialize="Sub-channel", serialize="SubChannel")]
	SubChannel = 0x170,
    #[strum(serialize="Sub-ChannelIncrement", serialize="SubChannelIncrement")]
	SubChannelIncrement = 0x171,
    #[strum(serialize="Sub-ChannelDecrement", serialize="SubChannelDecrement")]
	SubChannelDecrement = 0x172,
	AlternateAudioIncrement = 0x173,
	AlternateAudioDecrement = 0x174,
	ApplicationLaunchButtons = 0x180,
	ALLaunchButtonConfigurationTool = 0x181,
	ALProgrammableButtonConfiguration = 0x182,
    #[strum(serialize="ALConsumerControlConfiguration", serialize="Favorites")]
	ALConsumerControlConfiguration = 0x183,
	ALWordProcessor = 0x184,
	ALTextEditor = 0x185,
	ALSpreadsheet = 0x186,
	ALGraphicsEditor = 0x187,
	ALPresentationApp = 0x188,
	ALDatabaseApp = 0x189,
	ALEmailReader = 0x18A,
	ALNewsreader = 0x18B,
	ALVoicemail = 0x18C,
    #[strum(serialize="ALContacts/Address_Book", serialize="ALContactsAddressBook")]
	ALContactsAddressBook = 0x18D,
    #[strum(serialize="ALCalendar/Schedule", serialize="ALCalendarSchedule")]
	ALCalendarSchedule = 0x18E,
    #[strum(serialize="ALTask/ProjectManager", serialize="ALTaskProjectManager")]
	ALTaskProjectManager = 0x18F,
    #[strum(serialize="ALLog/Journal/Timecard", serialize="ALLogJournalTimecard")]
	ALLogJournalTimecard = 0x190,
    #[strum(serialize="ALCheckbook/Finance", serialize="ALCheckbookFinance")]
	ALCheckbookFinance = 0x191,
    #[strum(serialize="ALCalculator", serialize="Calculator")]
	ALCalculator = 0x192,
    #[strum(serialize="ALA/VCapture/Playback", serialize="ALAVCapturePlayback")]
	ALAVCapturePlayback = 0x193,
	ALLocalMachineBrowser = 0x194,
    #[strum(serialize="ALLAN/WANBrowser", serialize="ALLANWANBrowser")]
	ALLANWANBrowser = 0x195,
	ALInternetBrowser = 0x196,
    #[strum(serialize="ALRemoteNetworking/ISPConnect", serialize="ALRemoteNetworkingISPConnect")]
	ALRemoteNetworkingISPConnect = 0x197,
	ALNetworkConference = 0x198,
	ALNetworkChat = 0x199,
    #[strum(serialize="ALTelephonyDialer", serialize="ALTelephonyDialer")]
	ALTelephonyDialer = 0x19A,
	ALLogon = 0x19B,
	ALLogoff = 0x19C,
    #[strum(serialize="ALLogon/Logoff", serialize="ALLogonLogoff")]
	ALLogonLogoff = 0x19D,
    #[strum(serialize="ALTerminalLock/Screensaver", serialize="ALTerminalLockScreensaver", serialize="ScreenLock")]
	ALTerminalLockScreensaver = 0x19E,
	ALControlPanel = 0x19F,
    #[strum(serialize="ALCommandLineProcessor/Run", serialize="ALCommandLineProcessorRun")]
	ALCommandLineProcessorRun = 0x1A0,
    #[strum(serialize="ALProcess/Task_Manager", serialize="ALProcessTaskManager")]
	ALProcessTaskManager = 0x1A1,
    #[strum(serialize="ALSelectTask/Application", serialize="ALSelectTaskApplication")]
	ALSelectTaskApplication = 0x1A2,
    #[strum(serialize="ALNextTask/Application", serialize="ALNextTaskApplication")]
	ALNextTaskApplication = 0x1A3,
    #[strum(serialize="ALPreviousTask/Application", serialize="ALPreviousTaskApplication")]
	ALPreviousTaskApplication = 0x1A4,
    #[strum(serialize="ALPreemptiveHaltTask/Application", serialize="ALPreemptiveHaltTaskApplication")]
	ALPreemptiveHaltTaskApplication = 0x1A5,
	GenericGUIApplicationControls = 0x200,
	ACNew = 0x201,
	ACOpen = 0x202,
	ACClose = 0x203,
	ACExit = 0x204,
	ACMaximize = 0x205,
	ACMinimize = 0x206,
	ACSave = 0x207,
	ACPrint = 0x208,
	ACProperties = 0x209,
	ACUndo = 0x21A,
	ACCopy = 0x21B,
	ACCut = 0x21C,
	ACPaste = 0x21D,
	ACSelectAll = 0x21E,
	ACFind = 0x21F,
	ACFindAndReplace = 0x220,
	ACSearch = 0x221,
	ACGoTo = 0x222,
	ACHome = 0x223,
	ACBack = 0x224,
	ACForward = 0x225,
	ACStop = 0x226,
	ACRefresh = 0x227,
	ACPreviousLink = 0x228,
	ACNextLink = 0x229,
	ACBookmarks = 0x22A,
	ACHistory = 0x22B,
	ACSubscriptions = 0x22C,
	ACZoomIn = 0x22D,
	ACZoomOut = 0x22E,
	ACZoom = 0x22F,
	ACFullScreenView = 0x230,
	ACNormalView = 0x231,
	ACViewToggle = 0x232,
	ACScrollUp = 0x233,
	ACScrollDown = 0x234,
	ACScroll = 0x235,
	ACPanLeft = 0x236,
	ACPanRight = 0x237,
	ACPan = 0x238,
	ACNewWindow = 0x239,
	ACTileHorizontally = 0x23A,
	ACTileVertically = 0x23B,
	ACFormat = 0x23C,
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
