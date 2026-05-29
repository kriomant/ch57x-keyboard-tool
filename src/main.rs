mod config;
mod keyboard;
mod options;
mod parse;

use std::io::{BufReader, Read, StdinLock};

use crate::config::{Config, KeyboardModel};
use crate::keyboard::{
    k884x, k8890, Keyboard, KnobAction, MediaCode, Modifier,
    WellKnownCode,
};
use crate::options::{Command, LedCommand, TestLedCommand};
use crate::{keyboard::Key, options::Options};

use anyhow::{Result, anyhow, ensure};
use indoc::indoc;
use itertools::Itertools;
use log::debug;
use options::{ConfigParams, DevelOptions};
use rusb::{Context, Device, DeviceDescriptor, DeviceHandle, TransferType};

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
            println!(" - wheel(-100)");
            println!(" - click(left+right)");
            println!(" - move(5,0)");
            println!(" - drag(left+right,0,5)");
        }

        Command::Validate(params) => {
            let config: Config = load_config(&params)
                .context("load mapping config")?;
            let _ = config.render().context("render mappings config")?;
            println!("config is valid 👌")
        }

        Command::Upload(params) => {
            let config: Config = load_config(&params)
                .context("load mapping config")?;
            let configured_model = config.model;
            let (buttons, knobs) = (config.rows * config.columns, config.knobs);
            let layers = config.render().context("render mapping config")?;

            let (handle, endpoint, model) = open_device(&options.devel_options, configured_model)?;
            if configured_model.is_none() {
                eprintln!(
                    "⚠️ Configuration file does not specify `model`. This field will be required in a future version. Add `model: {model}`."
                );
            }
            let keyboard = create_driver(model, buttons, knobs)?;

            let mut output = Vec::new();

            // Apply keyboard mapping.
            for (layer_idx, layer) in layers.iter().enumerate() {
                for (button_idx, macro_) in layer.buttons.iter().enumerate() {
                    if let Some(macro_) = macro_ {
                        keyboard.bind_key(layer_idx as u8, Key::Button(button_idx as u8), macro_, &mut output)
                            .context("bind key")?;
                    }
                }

                for (knob_idx, knob) in layer.knobs.iter().enumerate() {
                    if let Some(macro_) = &knob.ccw {
                        keyboard.bind_key(layer_idx as u8, Key::Knob(knob_idx as u8, KnobAction::RotateCCW), macro_, &mut output)?;
                    }
                    if let Some(macro_) = &knob.press {
                        keyboard.bind_key(layer_idx as u8, Key::Knob(knob_idx as u8, KnobAction::Press), macro_, &mut output)?;
                    }
                    if let Some(macro_) = &knob.cw {
                        keyboard.bind_key(layer_idx as u8, Key::Knob(knob_idx as u8, KnobAction::RotateCW), macro_, &mut output)?;
                    }
                }
            }

            // Send all accumulated data to device
            send_to_device(&handle, endpoint, &output)?;
        }

        Command::Led(LedCommand { model, mut args }) => {
            let (handle, endpoint, model) = open_device(&options.devel_options, model)?;
            // TODO: fix this dirty hack
            let mut keyboard = create_driver(model, 0, 0)?;
            let mut output = Vec::new();

            args.insert(0, "led".to_string());
            keyboard.set_led(&args, &mut output)?;
            send_to_device(&handle, endpoint, &output)?;
        }

        Command::TestLed(TestLedCommand { model, mut args }) => {
            // Create driver without USB device initialization
            let mut keyboard = create_driver(model, 0, 0)?;
            let mut output = Vec::new();

            // Validate LED arguments
            args.insert(0, "test-led".to_string());
            keyboard.set_led(&args, &mut output)?;

            println!("Generated {} bytes of data", output.len());
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

fn send_to_device(handle: &DeviceHandle<Context>, endpoint: u8, output: &[u8]) -> Result<()> {
    use std::time::Duration;
    use log::debug;

    const DEFAULT_TIMEOUT: Duration = Duration::from_millis(100);

    // Process output buffer in 64-byte chunks
    for chunk in output.chunks(64) {
        debug!("send: {:02x?}", chunk);
        let written = handle.write_interrupt(endpoint, chunk, DEFAULT_TIMEOUT)?;
        ensure!(written == chunk.len(), "not all data written");
    }
    Ok(())
}

fn open_device(
    devel_options: &DevelOptions,
    configured_model: Option<KeyboardModel>,
) -> Result<(DeviceHandle<Context>, u8, KeyboardModel)> {
    // Find USB device based on the configured model or product id
    let (device, desc, model) = find_device(devel_options, configured_model).context("find USB device")?;

    ensure!(
        desc.num_configurations() == 1,
        "only one device configuration is expected"
    );

    let preferred_endpoint = match model {
        KeyboardModel::Ch57x_1 => {
            k884x::Keyboard884x::preferred_endpoint()
        }
        KeyboardModel::Ch57x_2 => k8890::Keyboard8890::preferred_endpoint(),
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

    Ok((handle, endpt_addr, model))
}

fn create_driver(model: KeyboardModel, buttons: u8, knobs: u8) -> Result<Box<dyn Keyboard>> {
    let keyboard: Box<dyn Keyboard> = match model {
        KeyboardModel::Ch57x_1 => {
            Box::new(k884x::Keyboard884x::new(buttons, knobs)?)
        }
        KeyboardModel::Ch57x_2 => {
            Box::new(k8890::Keyboard8890::new())
        }
    };
    Ok(keyboard)
}

fn find_device(
    devel_options: &DevelOptions,
    configured_model: Option<KeyboardModel>,
) -> Result<(Device<Context>, DeviceDescriptor, KeyboardModel)> {
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

        let device_address = (device.bus_number(), device.address());
        if let Some(address) = devel_options.address && device_address != address {
            continue;
        }

        if let Some(vid) = devel_options.vendor_id {
            if desc.vendor_id() != vid {
                continue;
            }
        }

        if let Some(pid) = devel_options.product_id {
            if desc.product_id() != pid {
                continue;
            }
        }

        let models = if let Some(model) = configured_model {
            let supported_pairs = model.supported_vid_pid();
            let device_vid = desc.vendor_id();
            let device_pid = desc.product_id();

            let supported = match (devel_options.vendor_id, devel_options.product_id) {
                (Some(_), Some(_)) => {
                    true
                },
                (Some(_), None) => {
                    // Vendor ID is specified by user, accept it, but check that Product ID is among
                    // supported by this model.
                    supported_pairs.iter().any(|(_, pid)| *pid == device_pid)
                },
                (None, Some(_)) => {
                    // Product ID is specified by user, accept it, but check that Vendor ID is among
                    // supported by this model.
                    supported_pairs.iter().any(|(vid, _)| *vid == device_vid)
                }
                (None, None) => {
                    supported_pairs.contains(&(device_vid, device_pid))
                }
            };

            if !supported {
                continue;
            }

            vec![model]
        } else {
            let models = KeyboardModel::from_vid_pid(desc.vendor_id(), desc.product_id());
            if models.is_empty() {
                continue;
            }
            models
        };

        found.push((device, desc, models));
    }

    match found.len() {
        0 => Err(anyhow!(
            "CH57x keyboard device not found. Use --vendor-id and/or --product-id to override."
        )),
        1 => {
            let (device, desc, models) = found.pop().unwrap();
            if models.len() == 1 {
                return Ok((device, desc, models[0]));
            }

            Err(anyhow!(indoc! {"
                Found device, but there are several different models which can use such product ID.
                You need to select model and:
                * write it to configuration file ('model' field) for 'upload' command
                * or provide `--model` argument for 'led' command
                Possible values inferred from product ID: {}
                If you are not sure which model to use, just try each and find one which works.
            "}, models.iter().map(|model| format!("{model}")).join("\n")))
        }
        _ => {
            Err(anyhow!(indoc! {"
                Several compatible devices are found.
                Unfortunately, this model of keyboard doesn't have serial number.
                So specify USB address using --address option.

                Addresses:
                {}
            "}, found.iter()
                     .map(|(device, _, _)| format!("{}:{}", device.bus_number(), device.address()))
                     .join("\n")))
        }
    }
}

fn load_config(params: &ConfigParams) -> Result<Config> {
    // Load and validate mapping.
    let mut stdin_reader: BufReader<StdinLock<'static>>;
    let mut file_reader: BufReader<std::fs::File>;
    let reader: &mut dyn Read = match &params.config_path {
        Some(path) => {
            let file = std::fs::File::open(path).context("open config file")?;
            file_reader = BufReader::new(file);
            &mut file_reader
        }
        None => {
            stdin_reader = BufReader::new(std::io::stdin().lock());
            &mut stdin_reader
        }
    };
    Ok(serde_yaml::from_reader(reader)?)
}
