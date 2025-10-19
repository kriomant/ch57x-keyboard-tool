use std::ffi::OsString;
use std::num::ParseIntError;

use clap::{Args, Parser, Subcommand};
use crate::consts::VENDOR_ID;
use crate::keyboard::LedColor;
use crate::parse;

#[derive(Parser)]
pub struct Options {
    #[command(subcommand)]
    pub command: Command,

    #[clap(flatten)]
    pub devel_options: DevelOptions,
}

#[derive(Args)]
#[clap(version, next_help_heading = "Internal options (use with caution)")]
pub struct DevelOptions {
    #[arg(long, default_value_t=VENDOR_ID, value_parser=hex_or_decimal)]
    pub vendor_id: u16,

    #[arg(long, value_parser=hex_or_decimal)]
    pub product_id: Option<u16>,

    #[arg(long, value_parser=parse_address)]
    pub address: Option<(u8, u8)>,

    #[arg(long)]
    pub endpoint_address: Option<u8>,

    #[arg(long)]
    pub interface_number: Option<u8>,
}

pub fn hex_or_decimal(s: &str) -> Result<u16, ParseIntError>
{
    if s.to_ascii_lowercase().starts_with("0x") {
        u16::from_str_radix(&s[2..], 16)
    } else {
        u16::from_str_radix(s, 10)
    }
}

fn parse_address(s: &str) -> std::result::Result<(u8, u8), nom::error::Error<String>> {
    parse::from_str(parse::address, s)
}

#[derive(Subcommand)]
pub enum Command {
    /// Show supported keys and modifiers
    ShowKeys,

    /// Validate key mappings config on stdin
    Validate(ConfigParams),

    /// Upload key mappings from stdin to device
    Upload(ConfigParams),

    /// Select LED backlight mode
    Led(LedCommand),
}

#[derive(Parser)]
pub struct ConfigParams {
    /// Path to config file to upload.
    /// If not given, read from stdin.
    pub config_path: Option<OsString>,
}

#[derive(Parser, Clone, Default, Debug)]
pub struct LedCommand {
    /// Index of LED modes
    /// --------0x8840----------
    /// 0 - LEDs off
    /// 1 - backlight always on with LedColor
    /// 2 - no backlight, shock with LedColor when key pressed
    /// 3 - no backlight, shock2 when LedColor when key pressed
    /// 4 - no backlight, light up key with LedColor when pressed
    /// 5 - backlight white always on
    /// --------0x8890---color is not supported-------
    /// 0 - LEDs off
    /// 1 - LED on for last pushed key
    /// 2 - cycle through colors & buttons
    #[clap(verbatim_doc_comment)]
    pub index: u8,

    // Layer to set the LED
    #[clap(default_value_t = 1)]
    pub layer: u8,

    // made this an option because the 884x supports color but the 8890
    // does not. defaults to Red, but since the 8890 does not accept
    // setting color, it just gets ignored
    /// Note: Not applicable for product id 0x8890
    /// Color to apply with mode
    #[arg(value_enum, verbatim_doc_comment)]
    pub led_color: Option<LedColor>,
}
