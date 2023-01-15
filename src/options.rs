use clap::{Parser, Subcommand};
use crate::consts::{VENDOR_ID, PRODUCT_ID};
use crate::parse;

#[derive(Parser)]
pub struct Options {
    #[command(subcommand)]
    pub command: Command,

    #[arg(long, default_value_t=VENDOR_ID)]
    pub vendor_id: u16,

    #[arg(long, default_value_t=PRODUCT_ID)]
    pub product_id: u16,

    #[arg(long, value_parser=parse_address)]
    pub address: Option<(u8, u8)>,
}

fn parse_address(s: &str) -> std::result::Result<(u8, u8), nom::error::Error<String>> {
    parse::from_str(parse::address, s)
}

#[derive(Subcommand)]
pub enum Command {
    /// Upload key mappings from stdin to device
    Upload,

    /// Select LED backlight mode
    Led(LedCommand),
}

#[derive(Parser)]
pub struct LedCommand {
    /// Index of LED mode (zero-based)
    pub index: u8,
}
