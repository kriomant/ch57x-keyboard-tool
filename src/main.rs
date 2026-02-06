use std::io::{BufReader, Read, StdinLock};

use ch57x_keyboard_tool::config::Config;
use ch57x_keyboard_tool::keyboard::{
    KnobAction, MediaCode, Modifier,
    WellKnownCode,
};
use ch57x_keyboard_tool::options::{Command, LedCommand, TestLedCommand};
use ch57x_keyboard_tool::keyboard::Key;
use ch57x_keyboard_tool::options::Options;

use anyhow::{Result, anyhow, Context as _};
use itertools::Itertools;
use ch57x_keyboard_tool::options::{ConfigParams};
use ch57x_keyboard_tool::{open_device, send_to_device, create_driver};

use clap::Parser as _;
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
            let (buttons, knobs) = (config.rows * config.columns, config.knobs);
            let layers = config.render().context("render mapping config")?;

            let (handle, endpoint, id_product) = open_device(&options.devel_options)?;
            let keyboard = create_driver(id_product, buttons, knobs)?;

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

        Command::Led(LedCommand { mut args }) => {
            let (handle, endpoint, id_product) = open_device(&options.devel_options)?;
            // TODO: fix this dirty hack
            let mut keyboard = create_driver(id_product, 0, 0)?;
            let mut output = Vec::new();

            args.insert(0, "led".to_string());
            keyboard.set_led(&args, &mut output)?;
            send_to_device(&handle, endpoint, &output)?;
        }

        Command::TestLed(TestLedCommand { mut args }) => {
            let product_id = options.devel_options.product_id
                .ok_or_else(|| anyhow!("test-led command requires --product-id to be specified"))?;

            // Create driver without USB device initialization
            let mut keyboard = create_driver(product_id, 0, 0)?;
            let mut output = Vec::new();

            // Validate LED arguments
            args.insert(0, "test-led".to_string());
            keyboard.set_led(&args, &mut output)?;

            println!("Generated {} bytes of data", output.len());
        }
    }

    Ok(())
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
