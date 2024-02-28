mod config;
mod consts;
mod keyboard;
mod options;
mod parse;

use crate::config::Config;
use crate::consts::PRODUCT_IDS;
use crate::keyboard::{
    k8840, k8880, Keyboard, KnobAction, MediaCode, Modifier, MouseAction, MouseButton,
    WellKnownCode,
};
use crate::options::{Command, LedCommand};
use crate::{keyboard::Key, options::Options};

use anyhow::{anyhow, ensure, Result};
use indoc::indoc;
use itertools::Itertools;
use log::debug;
use rusb::{Context, Device, DeviceDescriptor, TransferType};

use anyhow::Context as _;
use clap::Parser as _;
use rusb::UsbContext as _;
use strum::EnumMessage as _;
use strum::IntoEnumIterator as _;

fn main() -> Result<()> {
    env_logger::init();
    let options = Options::parse();

    match options.command {
        Command::ShowKeys => {
            println!("Modifiers: ");
            for m in Modifier::iter() {
                println!(" - {}", m.get_serializations().iter().join(" / "));
            }

            println!();
            println!("Keys:");
            for c in WellKnownCode::iter() {
                println!(" - {c}");
            }

            println!();
            println!("Custom key syntax (use decimal code): <110>");

            println!();
            println!("Media keys:");
            for c in MediaCode::iter() {
                println!(" - {}", c.get_serializations().iter().join(" / "));
            }

            println!();
            println!("Mouse actions:");
            println!(" - {}", MouseAction::WheelDown);
            println!(" - {}", MouseAction::WheelUp);
            for b in MouseButton::iter() {
                println!(" - {b}");
            }
        }

        Command::Validate => {
            // Load and validate mapping.
            let config: Config = serde_yaml::from_reader(std::io::stdin().lock())
                .context("load mapping config")?;
            let _ = config.render().context("render mappings config")?;
            println!("config is valid 👌")
        }

        Command::Upload => {
            // Load and validate mapping.
            let config: Config = serde_yaml::from_reader(std::io::stdin().lock())
                .context("load mapping config")?;
            let layers = config.render().context("render mapping config")?;

            let mut keyboard = open_keyboard(&options)?;

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
            let mut keyboard = open_keyboard(&options)?;
            keyboard.set_led(index)?;
        }
    }

    Ok(())
}

fn find_interface_and_endpoint(
    device: &Device<Context>,
    interface_num: Option<u8>,
    endpoint_addr: u8,
) -> Result<(u8, u8)> {
    let conf_desc = device
        .config_descriptor(0)
        .context("get config #0 descriptor")?;

    // Get the numbers of interfaces to explore
    let interface_nums = match interface_num {
        Some(iface_num) => vec![iface_num],
        None => conf_desc.interfaces().map(|iface| iface.number()).collect(),
    };

    for iface_num in interface_nums {
        debug!("Probing interface {iface_num}");

        // Look for an interface with the given number
        let intf = conf_desc
            .interfaces()
            .find(|iface| iface_num == iface.number())
            .ok_or_else(|| {
                anyhow!(
                    "interface #{} not found, interface numbers:\n{:#?}",
                    iface_num,
                    conf_desc.interfaces().map(|i| i.number()).format(", ")
                )
            })?;

        // Check that it's a HID device
        let intf_desc = intf.descriptors().exactly_one().map_err(|_| {
            anyhow!(
                "only one interface descriptor is expected, got:\n{:#?}",
                intf.descriptors().format("\n")
            )
        })?;
        ensure!(
            intf_desc.class_code() == 0x03
                && intf_desc.sub_class_code() == 0x00
                && intf_desc.protocol_code() == 0x00,
            "unexpected interface parameters: {:#?}",
            intf_desc
        );

        // Look for suitable endpoints
        if let Some(endpt_desc) = intf_desc.endpoint_descriptors().find(|ep| {
            ep.transfer_type() == TransferType::Interrupt && ep.address() == endpoint_addr
        }) {
            debug!("Found endpoint {endpt_desc:?}");
            return Ok((iface_num, endpt_desc.address()));
        }
    }

    Err(anyhow!("No valid interface/endpoint combination found!"))
}

fn open_keyboard(options: &Options) -> Result<Box<dyn Keyboard>> {
    // Find USB device based on the product id
    let (device, desc, id_product) = find_device(options).context("find USB device")?;

    ensure!(
        desc.num_configurations() == 1,
        "only one device configuration is expected"
    );

    // Find correct endpoint
    let (intf_num, endpt_addr) = find_interface_and_endpoint(
        &device,
        options.devel_options.interface_number,
        options.devel_options.endpoint_address,
    )?;

    // Open device.
    let mut handle = device.open().context("open USB device")?;
    let _ = handle.set_auto_detach_kernel_driver(true);
    handle
        .claim_interface(intf_num)
        .context("claim interface")?;

    match id_product {
        0x8840 => {
            k8840::Keyboard8840::new(handle, endpt_addr).map(|v| Box::new(v) as Box<dyn Keyboard>)
        }
        0x8880 => {
            k8880::Keyboard8880::new(handle, endpt_addr).map(|v| Box::new(v) as Box<dyn Keyboard>)
        }
        _ => unreachable!("This shouldn't happen!"),
    }
}

fn find_device(opts: &Options) -> Result<(Device<Context>, DeviceDescriptor, u16)> {
    let options = vec![
        #[cfg(windows)] rusb::UsbOption::use_usbdk(),
    ];
    let usb_context = rusb::Context::with_options(&options)?;

    let mut found = vec![];
    for device in usb_context.devices().context("get USB device list")?.iter() {
        let desc = device.device_descriptor().context("get USB device info")?;
        debug!(
            "Bus {:03} Device {:03} ID {:04x}:{:04x}",
            device.bus_number(),
            device.address(),
            desc.vendor_id(),
            desc.product_id()
        );
        let product_id = desc.product_id();
        if desc.vendor_id() == opts.devel_options.vendor_id
            && match opts.devel_options.product_id {
                Some(prod_id) => prod_id == product_id,
                None => PRODUCT_IDS.contains(&product_id),
            }
        {
            found.push((device, desc, product_id));
        }
    }

    match found.len() {
        0 => Err(anyhow!(
            "CH57x keyboard device not found. Use --vendor-id and --product-id to override settings."
        )),
        1 => Ok(found.pop().unwrap()),
        _ => {
            let mut addresses = vec![];
            for (device, desc, product_id) in found {
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
                if opts.devel_options.address.as_ref() == Some(&address) {
                    return Ok((device, desc, product_id))
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
