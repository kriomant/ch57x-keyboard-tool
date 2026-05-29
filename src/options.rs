use std::ffi::OsString;
use std::num::ParseIntError;

use clap::{Args, Parser, Subcommand};
use crate::config::KeyboardModel;
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
    #[arg(long, value_parser=hex_or_decimal)]
    pub vendor_id: Option<u16>,

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
    #[arg(long)]
    pub model: Option<KeyboardModel>,

    #[arg(num_args=0.., allow_hyphen_values=true)]
    pub args: Vec<String>,
}

#[derive(Parser)]
pub struct TestLedCommand {
    #[arg(long)]
    pub model: KeyboardModel,

    /// LED command arguments (layer and mode)
    #[arg(num_args=0.., allow_hyphen_values=true)]
    pub args: Vec<String>,
}
