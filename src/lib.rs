pub mod config;
pub mod consts;
pub mod keyboard;
pub mod options;
pub mod parse;

use anyhow::{anyhow, bail, Result, ensure, Context as _};
use indoc::indoc;
use itertools::Itertools;
use log::debug;
use rusb::{Context, Device, DeviceDescriptor, DeviceHandle, TransferType, UsbContext};
use crate::consts::PRODUCT_IDS;
use crate::keyboard::{k884x, k8890, Keyboard};
use crate::options::DevelOptions;

pub fn create_driver(id_product: u16, buttons: u8, knobs: u8) -> Result<Box<dyn Keyboard>> {
    let keyboard: Box<dyn Keyboard> = match id_product {
        0x8840 | 0x8842 | 0x8850 => {
            Box::new(k884x::Keyboard884x::new(buttons, knobs)?)
        }
        0x8890 => {
            Box::new(k8890::Keyboard8890::new())
        }
        _ => bail!("unsupported device"),
    };
    Ok(keyboard)
}

pub fn find_device(devel_options: &DevelOptions) -> Result<(Device<Context>, DeviceDescriptor, u16)> {
    let options = vec![
        #[cfg(windows)] rusb::UsbOption::use_usbdk(),
    ];
    let usb_context = rusb::Context::with_options(&options)?;

    let mut found = vec![];
    for device in usb_context.devices().context("get USB device list")?.iter() {
        let device: Device<Context> = device;
        let desc = device.device_descriptor().context("get USB device info")?;
        debug!(
            "Bus {:03} Device {:03} ID {:04x}:{:04x}",
            device.bus_number(),
            device.address(),
            desc.vendor_id(),
            desc.product_id()
        );
        let product_id = desc.product_id();
        if desc.vendor_id() == devel_options.vendor_id
            && match devel_options.product_id {
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
            for (device, _desc, product_id) in found {
                let device: Device<Context> = device;
                let address = (device.bus_number(), device.address());
                if devel_options.address.as_ref() == Some(&address) {
                    let desc = device.device_descriptor().context("get USB device info")?;
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

pub fn find_interface_and_endpoint(
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
            .find(|intf| iface_num == intf.number())
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

        // Look for suitable endpoints
        if let Some(endpt_desc) = intf_desc.endpoint_descriptors().find(|ep| {
            ep.transfer_type() == TransferType::Interrupt && ep.address() == endpoint_addr
        }) {
            debug!("Found endpoint {endpt_desc:?}");
            if intf_desc.class_code() == 0x03
                && intf_desc.sub_class_code() == 0x00
                && intf_desc.protocol_code() == 0x00
            {
                return Ok((iface_num, endpt_desc.address()));
            } else {
                debug!("unexpected interface parameters: {:#?}", intf_desc);
            }
        }
    }

    Err(anyhow!("No valid interface/endpoint combination found!"))
}

pub fn send_to_device(handle: &DeviceHandle<Context>, endpoint: u8, output: &[u8]) -> Result<()> {
    use std::time::Duration;
    const DEFAULT_TIMEOUT: Duration = Duration::from_millis(100);

    // Process output buffer in 64-byte chunks
    for chunk in output.chunks(64) {
        debug!("send: {:02x?}", chunk);
        let written = handle.write_interrupt(endpoint, chunk, DEFAULT_TIMEOUT)?;
        ensure!(written == chunk.len(), "not all data written");
    }
    Ok(())
}

pub fn open_device(devel_options: &DevelOptions) -> Result<(DeviceHandle<Context>, u8, u16)> {
    // Find USB device based on the product id
    let (device, desc, id_product) = find_device(devel_options).context("find USB device")?;

    ensure!(
        desc.num_configurations() == 1,
        "only one device configuration is expected"
    );

    let preferred_endpoint = match id_product {
        0x8840 | 0x8842 | 0x8850 => k884x::Keyboard884x::preferred_endpoint(),
        0x8890 => k8890::Keyboard8890::preferred_endpoint(),
        _ => unreachable!("unsupported device"),
    };

    // Find correct endpoint
    let (intf_num, endpt_addr) = find_interface_and_endpoint(
        &device,
        devel_options.interface_number,
        devel_options.endpoint_address.unwrap_or(preferred_endpoint),
    )?;

    // Open device.
    let mut handle = device.open().context("open USB device")?;
    let _ = handle.set_auto_detach_kernel_driver(true);
    handle
        .claim_interface(intf_num)
        .context("claim interface")?;

    // Initialize device.
    send_to_device(&handle, endpt_addr, &[0u8; 64])?;

    Ok((handle, endpt_addr, id_product))
}