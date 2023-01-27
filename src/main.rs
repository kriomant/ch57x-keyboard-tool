mod consts;
mod options;
mod keyboard;
mod config;
mod parse;

use crate::config::Config;
use crate::options::{Command, LedCommand};
use crate::{options::Options, keyboard::Key};
use crate::keyboard::{Keyboard, KnobAction};

use anyhow::{anyhow, ensure, Result};
use itertools::Itertools;
use log::debug;
use rusb::{Device, DeviceDescriptor, GlobalContext, TransferType};
use indoc::indoc;

use anyhow::Context as _;
use clap::Parser as _;

fn main() -> Result<()> {
    env_logger::init();
    let options = Options::parse();

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
    let mut keyboard = Keyboard::new(handle, endpt_desc.address()).context("init keyboard")?;

    match options.command {
        Command::Upload => {
            // Load and validate mapping.
            let config: Config = serde_yaml::from_reader(std::io::stdin().lock())
                .context("load mapping config")?;
            let layers = config.render()?;

            // Apply keyboard mapping.
            for (layer_idx, layer) in layers.iter().enumerate() {
                for (button_idx, macro_) in layer.buttons.iter().enumerate() {
                    if let Some(macro_) = macro_ {
                        keyboard.bind_key(layer_idx as u8, Key::Button(button_idx as u8), macro_)
                            .context("bind key")?;
                    }
                }

                for (knob_idx, knob) in layer.knobs.iter().enumerate() {
                    if let Some(macro_) = &knob.ccw {
                        keyboard.bind_key(layer_idx as u8, Key::Knob(knob_idx as u8, KnobAction::RotateCCW), macro_)?;
                    }
                    if let Some(macro_) = &knob.press {
                        keyboard.bind_key(layer_idx as u8, Key::Knob(knob_idx as u8, KnobAction::Press), macro_)?;
                    }
                    if let Some(macro_) = &knob.cw {
                        keyboard.bind_key(layer_idx as u8, Key::Knob(knob_idx as u8, KnobAction::RotateCW), macro_)?;
                    }
                }
            }
        }

        Command::Led(LedCommand { index }) => {
            keyboard.set_led(index)?;
        }
    }

    Ok(())
}

fn find_device(opts: &Options) -> Result<(Device<GlobalContext>, DeviceDescriptor)> {
    let mut found = vec![];
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
            found.push((device, desc));
            continue
        }
    }

    match found.len() {
        0 => Err(anyhow!(
            "CH57x keyboard device not found. Use --vendor-id and --product-id to override settings."
        )),
        1 => Ok(found.pop().unwrap()),
        _ => {
            let mut addresses = vec![];
            for (device, desc) in found {
                /*let handle = device.open().context("open device")?;
                let langs = handle.read_languages(DEFAULT_TIMEOUT).context("get langs")?;
                dbg!(&langs);
                let lang =
                    // First try to find US English language
                    langs.iter().find(|l| {
                        l.primary_language() == PrimaryLanguage::English &&
                        l.sub_language() == SubLanguage::UnitedStates
                    })
                    // Then any English sublanguage
                    .or_else(|| langs.iter().find(|l| l.primary_language() == PrimaryLanguage::English))
                    // Then just first available language
                    .or_else(|| langs.first())
                    // Ok, give up
                    .ok_or_else(|| anyhow!("No languages found"))?;
                dbg!(lang);
                let serial = handle.read_serial_number_string(*lang, &desc, DEFAULT_TIMEOUT)
                    .context("read serial")?;*/
                let address = (device.bus_number(), device.address());
                if opts.address.as_ref() == Some(&address) {
                    return Ok((device, desc))
                }

                addresses.push(address);
            }

            Err(anyhow!(indoc! {"
                Several compatible devices are found.
                Unfortunately, this model of keyboard doesn't have serial number.
                So specify USB address using --address option.
                
                Addresses:
                {}
            "}, addresses.iter().map(|(bus, addr)| format!("{bus}:{addr}")).join("\n")))
        }
    }
}
