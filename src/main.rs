mod consts;
mod options;
mod keyboard;
mod config;
mod parse;

use crate::config::Config;
use crate::{options::Options, keyboard::Key};
use crate::keyboard::Keyboard;

use anyhow::{anyhow, bail, ensure, Result};
use itertools::Itertools;
use log::debug;
use rusb::{Device, DeviceDescriptor, GlobalContext, TransferType};

use anyhow::Context as _;
use clap::Parser as _;

fn main() -> Result<()> {
    env_logger::init();
    let options = Options::parse();

    // Load and validate mapping.
    let config: Config = serde_yaml::from_reader(std::io::stdin().lock())
        .context("load mapping config")?;
    let layers = config.render()?;

    // Find USB device and endpoint.
    let (device, desc) = find_device(&options).context("find USB device")?;

    // Find device endpoint.
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

    // Open device.
    let mut handle = device.open().context("open USB device")?;
    handle.claim_interface(intf.number())?;

    // Apply keyboard mapping.
    let mut keyboard = Keyboard::new(handle, endpt_desc.address()).context("init keyboard")?;
    for (layer_idx, layer) in layers.iter().enumerate() {
        for (button_idx, macro_) in layer.buttons.iter().enumerate() {
            if let Some(macro_) = macro_ {
                keyboard.bind_key(layer_idx as u8, Key::Button(button_idx as u8), macro_)
                    .context("bind key")?;
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        }
    }

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
