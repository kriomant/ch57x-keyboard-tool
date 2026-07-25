use std::ffi::OsString;
use std::num::ParseIntError;

use clap::{Args, Parser, Subcommand, ValueEnum};
use crate::consts::VENDOR_ID;
use crate::parse;

/// Protocol implementation to talk to the keyboard with.
///
/// Vendors reuse the same USB IDs across physically different pads that do not
/// share a protocol, so the ID alone cannot always pick the right driver. This
/// lets the choice be overridden when auto-selection guesses wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Driver {
    /// Product IDs 8840, 8842 and most 8850 pads.
    K884x,
    /// The 8850 variant using command byte 0xfd and 3-byte argument groups.
    K8850,
    /// Product ID 8890.
    K8890,
}

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

    /// Override which protocol implementation is used.
    #[arg(long, value_enum)]
    pub driver: Option<Driver>,
}

pub fn hex_or_decimal(s: &str) -> Result<u16, ParseIntError>
{
    if s.to_ascii_lowercase().starts_with("0x") {
        u16::from_str_radix(&s[2..], 16)
    } else {
        #[allow(clippy::from_str_radix_10)] // For consistency with code above
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

    /// Test LED mode specification without device initialization (hidden command)
    #[command(hide = true)]
    TestLed(TestLedCommand),
}

#[derive(Parser)]
pub struct ConfigParams {
    /// Path to config file to upload.
    /// If not given, read from stdin.
    pub config_path: Option<OsString>,
}

#[derive(Parser)]
pub struct LedCommand {
    #[arg(num_args=0.., allow_hyphen_values=true)]
    pub args: Vec<String>,
}

#[derive(Parser)]
pub struct TestLedCommand {
    /// LED command arguments (layer and mode)
    #[arg(num_args=0.., allow_hyphen_values=true)]
    pub args: Vec<String>,
}
