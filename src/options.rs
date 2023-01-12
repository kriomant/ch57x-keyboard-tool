use clap::Parser;
use crate::consts::{VENDOR_ID, PRODUCT_ID};

#[derive(Parser)]
pub struct Options {
    #[arg(long, default_value_t=VENDOR_ID)]
    pub vendor_id: u16,

    #[arg(long, default_value_t=PRODUCT_ID)]
    pub product_id: u16,
}
