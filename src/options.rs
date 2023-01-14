use clap::{Parser, Subcommand};
use crate::consts::{VENDOR_ID, PRODUCT_ID};

#[derive(Parser)]
pub struct Options {
    #[command(subcommand)]
    pub command: Command,

    #[arg(long, default_value_t=VENDOR_ID)]
    pub vendor_id: u16,

    #[arg(long, default_value_t=PRODUCT_ID)]
    pub product_id: u16,
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
