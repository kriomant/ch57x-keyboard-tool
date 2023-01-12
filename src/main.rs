mod consts;
mod options;
mod keyboard;

use crate::{options::Options, keyboard::Key};
use crate::keyboard::{Keyboard, Macro, Code};

use anyhow::{anyhow, bail, ensure, Result};
use enumset::EnumSet;
use itertools::Itertools;
use log::debug;
use rusb::{Device, DeviceDescriptor, GlobalContext, TransferType};

use anyhow::Context as _;
use clap::Parser as _;

fn main() -> Result<()> {
    env_logger::init();
    let options = Options::parse();

    let (device, desc) = find_device(&options).context("find USB device")?;

    ensure!(
        desc.num_configurations() == 1,
        "only one device configuration is expected"
    );
    let conf_desc = device
        .config_descriptor(0)
        .context("get config #0 descriptor")?;
    let intf = conf_desc
        .interfaces()
        .find(|intf| intf.number() == 1)
        .ok_or_else(|| anyhow!("interface #1 not found"))?;
    let intf_desc = intf
        .descriptors()
        .exactly_one()
        .map_err(|_| anyhow!("only one interface descriptor is expected"))?;
    ensure!(
        intf_desc.class_code() == 0x03
            && intf_desc.sub_class_code() == 0x00
            && intf_desc.protocol_code() == 0x00,
        "unexpected interface parameters"
    );
    let endpt_desc = intf_desc
        .endpoint_descriptors()
        .exactly_one()
        .map_err(|_| anyhow!("single endpoint is expected"))?;
    ensure!(
        endpt_desc.transfer_type() == TransferType::Interrupt,
        "unexpected endpoint transfer type"
    );
    let endpt_addr = endpt_desc.address();

    let mut handle = device.open().context("open USB device")?;
    handle.claim_interface(intf.number())?;

    let mut keyboard = Keyboard::new(handle, endpt_addr).context("init keyboard")?;
    keyboard.bind_key(0, Key::Button(0), &Macro::Keyboard(vec![
        (EnumSet::empty(), Code::B),
    ])).context("bind key")?;

    Ok(())
}

fn find_device(opts: &Options) -> Result<(Device<GlobalContext>, DeviceDescriptor)> {
    for device in rusb::devices().context("get USB device list")?.iter() {
        let desc = device.device_descriptor().context("get USB device info")?;
        debug!(
            "Bus {:03} Device {:03} ID {:04x}:{:04x}",
            device.bus_number(),
            device.address(),
            desc.vendor_id(),
            desc.product_id()
        );
        if desc.vendor_id() == opts.vendor_id && desc.product_id() == opts.product_id {
            return Ok((device, desc));
        }
    }

    bail!(
        "CH57x keyboard device not found. Use --vendor-id and --product-id to override settings."
    );
}
