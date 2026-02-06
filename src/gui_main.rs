use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar, ViewStack, ViewSwitcher};
use gtk4 as gtk;
use gtk::{Box, Orientation, Button, Label, Grid, Entry, FileChooserDialog, ResponseType, ComboBoxText, Dialog, EventControllerKey, SpinButton, Adjustment, TextView, ScrolledWindow, TextTagTable, TextBuffer, CssProvider};
use ch57x_keyboard_tool::config::{Config, Layer, Knob, Orientation as KbdOrientation};
use ch57x_keyboard_tool::keyboard::{Key, KnobAction, Macro, Accord, Code, MediaCode, MouseEvent, MouseAction, MouseButton, Modifier, KeyboardEvent, WellKnownCode, MacroOptions};
use rusb::{Device, Context, DeviceDescriptor};
use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use std::fs::File;
use enumset::EnumSet;
use directories::ProjectDirs;
use ch57x_keyboard_tool::{find_device, create_driver};
use ch57x_keyboard_tool::options::DevelOptions;

struct AppState {
    config: Config,
    last_saved_yml: String,
    view_stack: ViewStack,
    status_label: Label,
    debug_buffer: TextBuffer,
    yml_buffer: TextBuffer,
    yml_container: Box,
}

fn main() {
    let app = Application::builder()
        .application_id("com.github.kriomant.ch57x-keyboard-gui")
        .build();

    app.connect_startup(|_| {
        let provider = CssProvider::new();
        provider.load_from_data("
            .yml-preview-saved { border: 2px solid #2ec27e; border-radius: 6px; padding: 4px; }
            .yml-preview-modified { border: 2px solid #e01b24; border-radius: 6px; padding: 4px; }
        ");
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.connect_activate(build_ui);
    app.run();
}

fn log_debug(buffer: &TextBuffer, msg: &str) {
    let mut end_iter = buffer.end_iter();
    buffer.insert(&mut end_iter, &format!("[{}] {}\n", chrono::Local::now().format("%H:%M:%S"), msg));
}

fn update_yml_preview(state: &Arc<Mutex<AppState>>) {
    let state = state.lock().unwrap();
    let current_yml = serde_yaml::to_string(&state.config).unwrap_or_default();
    state.yml_buffer.set_text(&current_yml);
    
    if current_yml.trim() == state.last_saved_yml.trim() {
        state.yml_container.add_css_class("yml-preview-saved");
        state.yml_container.remove_css_class("yml-preview-modified");
    } else {
        state.yml_container.add_css_class("yml-preview-modified");
        state.yml_container.remove_css_class("yml-preview-saved");
    }
}

fn get_default_config_path() -> Option<std::path::PathBuf> {
    ProjectDirs::from("com.github", "kriomant", "ch57x-keyboard")
        .map(|dirs| dirs.config_dir().join("cfg.yml"))
}

fn build_ui(app: &Application) {
    // Determine startup config
    let config_path = get_default_config_path();
    let config = if let Some(ref path) = config_path {
        if path.exists() {
            load_config_from_path(path).unwrap_or_else(|_| create_default_config())
        } else {
            create_default_config()
        }
    } else {
        create_default_config()
    };

    let last_saved_yml = serde_yaml::to_string(&config).unwrap_or_default();

    // If it didn't exist, try to save it as default
    if let Some(ref path) = config_path {
        if !path.exists() {
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            let _ = save_config_to_path(&config, path);
        }
    }

    let view_stack = ViewStack::new();
    let status_label = Label::new(Some("Initializing..."));
    let debug_buffer = TextBuffer::new(None::<&TextTagTable>);
    let yml_buffer = TextBuffer::new(None::<&TextTagTable>);
    let yml_container = Box::new(Orientation::Vertical, 0);
    
    let state = Arc::new(Mutex::new(AppState { 
        config, 
        last_saved_yml,
        view_stack: view_stack.clone(),
        status_label: status_label.clone(),
        debug_buffer: debug_buffer.clone(),
        yml_buffer: yml_buffer.clone(),
        yml_container: yml_container.clone(),
    }));

    // Log startup info
    if let Some(ref path) = config_path {
        log_debug(&debug_buffer, &format!("Config path: {}", path.display()));
    }

    let content = Box::new(Orientation::Vertical, 0);

    let header_bar = HeaderBar::new();
    let switcher = ViewSwitcher::new();
    switcher.set_stack(Some(&view_stack));
    header_bar.set_title_widget(Some(&switcher));
    
    let add_layer_btn = Button::with_label("+ Layer");
    let state_clone = state.clone();
    let view_stack_clone = view_stack.clone();
    add_layer_btn.connect_clicked(move |_| {
        {
            let mut state = state_clone.lock().unwrap();
            let rows = state.config.rows;
            let cols = state.config.columns;
            let knobs = state.config.knobs;
            state.config.layers.push(Layer {
                buttons: vec![vec![None; cols as usize]; rows as usize],
                knobs: vec![Knob { ccw: None, press: None, cw: None }; knobs as usize],
            });
            log_debug(&state.debug_buffer, "Layer added.");
        }
        refresh_view_stack(&view_stack_clone, &state_clone);
        update_yml_preview(&state_clone);
    });
    header_bar.pack_start(&add_layer_btn);

    let remove_layer_btn = Button::with_label("- Layer");
    let state_clone = state.clone();
    let view_stack_clone = view_stack.clone();
    remove_layer_btn.connect_clicked(move |_| {
        let mut removed = false;
        {
            let mut state = state_clone.lock().unwrap();
            if state.config.layers.len() > 1 {
                state.config.layers.pop();
                removed = true;
                log_debug(&state.debug_buffer, "Layer removed.");
            }
        }
        if removed {
            refresh_view_stack(&view_stack_clone, &state_clone);
            update_yml_preview(&state_clone);
        }
    });
    header_bar.pack_start(&remove_layer_btn);

    content.append(&header_bar);

    let main_box = Box::new(Orientation::Vertical, 12);
    main_box.set_margin_top(12);
    main_box.set_margin_bottom(12);
    main_box.set_margin_start(12);
    main_box.set_margin_end(12);
    
    // Status and diagnostic box
    let diag_box = Box::new(Orientation::Horizontal, 12);
    diag_box.set_halign(gtk::Align::Center);
    let diag_icon = Label::new(None);
    diag_icon.add_css_class("title-1");
    
    let refresh_btn = Button::builder()
        .label("🔄 Refresh Status")
        .build();

    diag_box.append(&diag_icon);
    diag_box.append(&status_label);
    diag_box.append(&refresh_btn);
    main_box.append(&diag_box);

    let button_box = Box::new(Orientation::Horizontal, 6);
    button_box.set_halign(gtk::Align::Center);
    
    let load_button = Button::with_label("Load Config...");
    let state_clone = state.clone();
    load_button.connect_clicked(move |btn| {
        let window = btn.root().and_downcast::<gtk::Window>().unwrap();
        let dialog = FileChooserDialog::new(
            Some("Open Configuration"),
            Some(&window),
            gtk::FileChooserAction::Open,
            &[("Open", ResponseType::Accept), ("Cancel", ResponseType::Cancel)],
        );

        let state_clone = state_clone.clone();
        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        match load_config_from_path(&path) {
                            Ok(config) => {
                                let (status_label, view_stack, debug_buf) = {
                                    let mut state = state_clone.lock().unwrap();
                                    state.config = config.clone();
                                    state.last_saved_yml = serde_yaml::to_string(&config).unwrap_or_default();
                                    (state.status_label.clone(), state.view_stack.clone(), state.debug_buffer.clone())
                                };
                                refresh_view_stack(&view_stack, &state_clone);
                                update_yml_preview(&state_clone);
                                status_label.set_text(&format!("Loaded {}", path.display()));
                                log_debug(&debug_buf, &format!("Loaded config from {}", path.display()));
                            }
                            Err(e) => {
                                let state = state_clone.lock().unwrap();
                                state.status_label.set_text(&format!("Load failed: {}", e));
                                log_debug(&state.debug_buffer, &format!("Load failed: {}", e));
                            }
                        }
                    }
                }
            }
            d.destroy();
        });
        dialog.present();
    });
    button_box.append(&load_button);

    let save_button = Button::with_label("Save Config...");
    let state_clone = state.clone();
    save_button.connect_clicked(move |btn| {
        let window = btn.root().and_downcast::<gtk::Window>().unwrap();
        let dialog = FileChooserDialog::new(
            Some("Save Configuration"),
            Some(&window),
            gtk::FileChooserAction::Save,
            &[("Save", ResponseType::Accept), ("Cancel", ResponseType::Cancel)],
        );

        let state_clone = state_clone.clone();
        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        let config = state_clone.lock().unwrap().config.clone();
                        match save_config_to_path(&config, &path) {
                            Ok(_) => {
                                {
                                    let mut state = state_clone.lock().unwrap();
                                    state.last_saved_yml = serde_yaml::to_string(&config).unwrap_or_default();
                                    state.status_label.set_text(&format!("Saved to {}", path.display()));
                                }
                                update_yml_preview(&state_clone);
                                log_debug(&state_clone.lock().unwrap().debug_buffer, &format!("Saved config to {}", path.display()));
                            },
                            Err(e) => {
                                let state = state_clone.lock().unwrap();
                                state.status_label.set_text(&format!("Save failed: {}", e));
                                log_debug(&state.debug_buffer, &format!("Save failed: {}", e));
                            }
                        }
                    }
                }
            }
            d.destroy();
        });
        dialog.present();
    });
    button_box.append(&save_button);

    let upload_button = Button::with_label("Upload to Keyboard");
    upload_button.add_css_class("suggested-action");
    let state_clone = state.clone();
    upload_button.connect_clicked(move |_| {
        let config = state_clone.lock().unwrap().config.clone();
        log_debug(&state_clone.lock().unwrap().debug_buffer, "Starting upload...");
        match upload_config(&config) {
            Ok(_) => {
                {
                    let mut state = state_clone.lock().unwrap();
                    state.status_label.set_text("Upload successful");
                    log_debug(&state.debug_buffer, "Upload completed successfully.");
                    // Also save to default config path on successful upload
                    if let Some(path) = get_default_config_path() {
                        let _ = save_config_to_path(&state.config, &path);
                        state.last_saved_yml = serde_yaml::to_string(&state.config).unwrap_or_default();
                        log_debug(&state.debug_buffer, "Config auto-saved to ~/.config/ch57x-keyboard/cfg.yml");
                    }
                }
                update_yml_preview(&state_clone);
            },
            Err(e) => {
                let state = state_clone.lock().unwrap();
                state.status_label.set_text(&format!("Upload failed: {}", e));
                log_debug(&state.debug_buffer, &format!("UPLOAD ERROR: {}", e));
            },
        }
    });
    button_box.append(&upload_button);
    
    main_box.append(&button_box);

    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let state_for_rx = state.clone();
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        while let Ok(msg) = rx.try_recv() {
            let state = state_for_rx.lock().unwrap();
            state.status_label.set_text(&msg);
            log_debug(&state.debug_buffer, &msg);
        }
        gtk::glib::ControlFlow::Continue
    });

    let fix_perms_btn = Button::with_label("Fix Linux Permissions");
    let tx_clone = tx.clone();
    fix_perms_btn.connect_clicked(move |_| {
        let tx = tx_clone.clone();
        std::thread::spawn(move || {
            let _ = tx.send("Requesting root permissions to fix udev rules...".to_string());
            
            let cmd = "printf \"SUBSYSTEM==\\\"usb\\\", ATTR{idVendor}==\\\"1189\\\", ATTR{idProduct}==\\\"8890\\\", MODE=\\\"0666\\\", TAG+=\\\"uaccess\\\"\\nSUBSYSTEM==\\\"hidraw\\\", ATTRS{idVendor}==\\\"1189\\\", ATTRS{idProduct}==\\\"8890\\\", MODE=\\\"0666\\\", TAG+=\\\"uaccess\\\"\\n\" > /etc/udev/rules.d/50-ch57x-keyboard.rules && udevadm control --reload-rules && udevadm trigger";
            
            let result = std::process::Command::new("pkexec").arg("bash").arg("-c").arg(cmd).status();

            match result {
                Ok(status) if status.success() => {
                    let _ = tx.send("Permissions fixed. PLEASE UNPLUG AND RE-PLUG YOUR KEYBOARD NOW.".to_string());
                }
                Ok(s) => {
                    let _ = tx.send(format!("Failed to apply rules: {}", s));
                }
                Err(e) => {
                    let _ = tx.send(format!("Error: {}. Ensure 'pkexec' is installed.", e));
                }
            }
        });
    });
    button_box.append(&fix_perms_btn);

    let led_box = Box::new(Orientation::Horizontal, 6);
    led_box.set_halign(gtk::Align::Center);
    led_box.append(&Label::new(Some("LED Mode:")));
    let led_combo = ComboBoxText::new();
    for i in 0..10 {
        led_combo.append_text(&format!("Mode {}", i));
    }
    led_combo.set_active(Some(0));
    
    let state_clone = state.clone();
    let led_apply_button = Button::with_label("Apply LED");
    let led_combo_clone = led_combo.clone();
    led_apply_button.connect_clicked(move |_| {
        let active = led_combo_clone.active().unwrap_or(0) as u8;
        let state = state_clone.lock().unwrap();
        state.status_label.set_text("Setting LED...");
        match set_keyboard_led(active) {
            Ok(_) => {
                state.status_label.set_text(&format!("LED set to Mode {}", active));
                log_debug(&state.debug_buffer, &format!("LED mode {} applied.", active));
            },
            Err(e) => {
                state.status_label.set_text(&format!("LED failed: {}", e));
                log_debug(&state.debug_buffer, &format!("LED ERROR: {}", e));
            }
        }
    });
    led_box.append(&led_combo);
    led_box.append(&led_apply_button);
    main_box.append(&led_box);

    main_box.append(&view_stack);

    main_box.append(&gtk::Separator::new(Orientation::Horizontal));
    main_box.append(&Label::new(Some("Diagnostic Console:")));
    let debug_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_height(100)
        .build();
    let debug_view = TextView::builder()
        .buffer(&debug_buffer)
        .editable(false)
        .cursor_visible(false)
        .wrap_mode(gtk::WrapMode::Word)
        .build();
    debug_view.add_css_class("monospace");
    debug_scroll.set_child(Some(&debug_view));
    main_box.append(&debug_scroll);

    // YML Preview
    main_box.append(&Label::new(Some("YML Preview (Live):")));
    let yml_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_height(200)
        .build();
    let yml_view = TextView::builder()
        .buffer(&yml_buffer)
        .editable(false)
        .cursor_visible(false)
        .build();
    yml_view.add_css_class("monospace");
    yml_scroll.set_child(Some(&yml_view));
    
    yml_container.set_margin_top(6);
    yml_container.set_margin_bottom(6);
    yml_container.append(&yml_scroll);
    main_box.append(&yml_container);

    refresh_view_stack(&view_stack, &state);
    update_yml_preview(&state);

    let state_diag = state.clone();
    let diag_icon_clone = diag_icon.clone();
    
    let run_diagnostic = move || {
        let res = find_keyboard();
        let log_msg;
        
        let (icon, msg) = match res {
            Ok((device, _desc, _addr)) => {
                let bus = device.bus_number();
                let port = device.address();
                let path = format!("/dev/bus/usb/{:03}/{:03}", bus, port);
                
                let rule_path = "/etc/udev/rules.d/50-ch57x-keyboard.rules";
                let rule_info = if std::path::Path::new(rule_path).exists() {
                    match std::fs::read_to_string(rule_path) {
                        Ok(content) => format!("Found udev rule at {}:\n{}", rule_path, content.trim()),
                        Err(e) => format!("Found udev rule at {} but could not read: {}", rule_path, e),
                    }
                } else {
                    "No udev rule found at /etc/udev/rules.d/50-ch57x-keyboard.rules".to_string()
                };

                match device.open() {
                    Ok(_) => {
                        log_msg = format!("Diagnostic: Keyboard found at {}. R/W OK.\n{}", path, rule_info);
                        (
                            "✅", 
                            format!("Keyboard detected at {}. Ready.", path),
                        )
                    },
                    Err(_) => {
                        log_msg = format!("Diagnostic: Keyboard found at {} but ACCESS DENIED.\n{}", path, rule_info);
                        (
                            "⚠️", 
                            "Permissions Denied! Click 'Fix Linux Permissions' and re-plug.".to_string(),
                        )
                    }
                }
            },
            Err(_) => {
                log_msg = "Diagnostic: Searching for CH57x keyboard... Not found.".to_string();
                (
                    "❌", 
                    "Keyboard not found. Connect it via USB.".to_string(),
                )
            }
        };

        diag_icon_clone.set_text(icon);
        let state = state_diag.lock().unwrap();
        state.status_label.set_text(&msg);
        log_debug(&state.debug_buffer, &log_msg);
    };

    run_diagnostic();

    let run_diagnostic_clone = run_diagnostic.clone();
    refresh_btn.connect_clicked(move |_| {
        run_diagnostic_clone();
    });

    content.append(&main_box);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("CH57x Keyboard Tool")
        .default_width(900)
        .default_height(900)
        .content(&content)
        .build();

    window.present();
}

fn create_default_config() -> Config {
    Config {
        orientation: KbdOrientation::Normal,
        rows: 3,
        columns: 4,
        knobs: 2,
        layers: vec![
            Layer {
                buttons: vec![vec![None; 4]; 3],
                knobs: vec![Knob { ccw: None, press: None, cw: None }; 2],
            }
        ],
    }
}

fn load_config_from_path(path: &std::path::Path) -> Result<Config> {
    let file = File::open(path)?;
    let config: Config = serde_yaml::from_reader(file)?;
    Ok(config)
}

fn save_config_to_path(config: &Config, path: &std::path::Path) -> Result<()> {
    let file = File::create(path)?;
    serde_yaml::to_writer(file, config)?;
    Ok(())
}

fn refresh_view_stack(view_stack: &ViewStack, state: &Arc<Mutex<AppState>>) {
    while let Some(child) = view_stack.first_child() {
        view_stack.remove(&child);
    }

    let hw_box = Box::new(Orientation::Vertical, 12);
    hw_box.set_margin_top(12);
    hw_box.append(&Label::new(Some("Hardware Settings")));
    
    let hw_grid = Grid::new();
    hw_grid.set_column_spacing(12);
    hw_grid.set_row_spacing(12);
    hw_grid.set_halign(gtk::Align::Center);

    let (rows, cols, knobs, orient) = {
        let state = state.lock().unwrap();
        (state.config.rows, state.config.columns, state.config.knobs, state.config.orientation)
    };

    let rows_adj = Adjustment::new(rows as f64, 1.0, 16.0, 1.0, 1.0, 0.0);
    let rows_spin = SpinButton::new(Some(&rows_adj), 1.0, 0);
    hw_grid.attach(&Label::new(Some("Rows:")), 0, 0, 1, 1);
    hw_grid.attach(&rows_spin, 1, 0, 1, 1);

    let cols_adj = Adjustment::new(cols as f64, 1.0, 16.0, 1.0, 1.0, 0.0);
    let cols_spin = SpinButton::new(Some(&cols_adj), 1.0, 0);
    hw_grid.attach(&Label::new(Some("Columns:")), 0, 1, 1, 1);
    hw_grid.attach(&cols_spin, 1, 1, 1, 1);

    let knobs_adj = Adjustment::new(knobs as f64, 0.0, 8.0, 1.0, 1.0, 0.0);
    let knobs_spin = SpinButton::new(Some(&knobs_adj), 1.0, 0);
    hw_grid.attach(&Label::new(Some("Knobs:")), 0, 2, 1, 1);
    hw_grid.attach(&knobs_spin, 1, 2, 1, 1);

    let orient_combo = ComboBoxText::new();
    orient_combo.append_text("Normal");
    orient_combo.append_text("Upside Down");
    orient_combo.append_text("Clockwise");
    orient_combo.append_text("Counter Clockwise");
    orient_combo.set_active(Some(match orient {
        KbdOrientation::Normal => 0,
        KbdOrientation::UpsideDown => 1,
        KbdOrientation::Clockwise => 2,
        KbdOrientation::CounterClockwise => 3,
    }));
    hw_grid.attach(&Label::new(Some("Orientation:")), 0, 3, 1, 1);
    hw_grid.attach(&orient_combo, 1, 3, 1, 1);

    let apply_hw_btn = Button::with_label("Apply Hardware Settings");
    let state_clone = state.clone();
    let view_stack_clone = view_stack.clone();
    apply_hw_btn.connect_clicked(move |_| {
        {
            let mut state = state_clone.lock().unwrap();
            state.config.rows = rows_spin.value() as u8;
            state.config.columns = cols_spin.value() as u8;
            state.config.knobs = knobs_spin.value() as u8;
            state.config.orientation = match orient_combo.active() {
                Some(0) => KbdOrientation::Normal,
                Some(1) => KbdOrientation::UpsideDown,
                Some(2) => KbdOrientation::Clockwise,
                Some(3) => KbdOrientation::CounterClockwise,
                _ => KbdOrientation::Normal,
            };
            let rows = state.config.rows as usize;
            let cols = state.config.columns as usize;
            let knobs = state.config.knobs as usize;
            for layer in &mut state.config.layers {
                layer.buttons = vec![vec![None; cols]; rows];
                layer.knobs = vec![Knob { ccw: None, press: None, cw: None }; knobs];
            }
            log_debug(&state.debug_buffer, "Hardware settings applied. Grid re-initialized.");
        }
        refresh_view_stack(&view_stack_clone, &state_clone);
        update_yml_preview(&state_clone);
    });
    hw_box.append(&hw_grid);
    hw_box.append(&apply_hw_btn);

    view_stack.add_titled(&hw_box, Some("hardware"), "Hardware");

    let config = {
        let state = state.lock().unwrap();
        state.config.clone()
    };

    for (i, layer) in config.layers.iter().enumerate() {
        let grid = Grid::new();
        grid.set_column_spacing(6);
        grid.set_row_spacing(6);
        grid.set_halign(gtk::Align::Center);

        for r in 0..layer.buttons.len() {
            for c in 0..layer.buttons[r].len() {
                let macro_val = layer.buttons[r][c].as_ref().map(|m| m.to_string()).unwrap_or_default();
                let entry = Entry::builder()
                    .text(&macro_val)
                    .placeholder_text(&format!("Btn {},{}", r, c))
                    .build();
                
                let state_clone = state.clone();
                let layer_idx = i;
                let row = r;
                let col = c;
                let entry_clone = entry.clone();
                entry.connect_changed(move |e| {
                    let text = e.text();
                    {
                        let mut state = state_clone.lock().unwrap();
                        if let Ok(m) = Macro::from_str(&text) {
                            state.config.layers[layer_idx].buttons[row][col] = Some(m);
                        } else if text.is_empty() {
                            state.config.layers[layer_idx].buttons[row][col] = None;
                        }
                    }
                    update_yml_preview(&state_clone);
                });

                let config_btn = Button::with_label("⚙");
                let entry_for_btn = entry_clone.clone();
                config_btn.connect_clicked(move |btn| {
                    let window = btn.root().and_downcast::<gtk::Window>().unwrap();
                    let entry_for_callback = entry_for_btn.clone();
                    show_macro_builder(&window, move |m| {
                        entry_for_callback.set_text(&m.to_string());
                    });
                });

                let hbox = Box::new(Orientation::Horizontal, 2);
                hbox.append(&entry_clone);
                hbox.append(&config_btn);

                grid.attach(&hbox, c as i32, r as i32, 1, 1);
            }
        }

        let knob_box = Box::new(Orientation::Horizontal, 12);
        knob_box.set_halign(gtk::Align::Center);
        for (k_idx, knob) in layer.knobs.iter().enumerate() {
            let k_vbox = Box::new(Orientation::Vertical, 4);
            k_vbox.append(&Label::new(Some(&format!("Knob {}", k_idx))));
            
            for (label, action) in [("CCW", "ccw"), ("Press", "press"), ("CW", "cw")] {
                let m_val = match action {
                    "ccw" => knob.ccw.as_ref(),
                    "press" => knob.press.as_ref(),
                    "cw" => knob.cw.as_ref(),
                    _ => None,
                }.map(|m| m.to_string()).unwrap_or_default();

                let entry = Entry::builder()
                    .text(&m_val)
                    .placeholder_text(label)
                    .build();
                
                let state_clone = state.clone();
                let layer_idx = i;
                let knob_idx = k_idx;
                let action_str = action.to_string();
                let entry_clone = entry.clone();
                entry.connect_changed(move |e| {
                    let text = e.text();
                    {
                        let mut state = state_clone.lock().unwrap();
                        let m = if text.is_empty() { None } else { Macro::from_str(&text).ok() };
                        match action_str.as_str() {
                            "ccw" => state.config.layers[layer_idx].knobs[knob_idx].ccw = m,
                            "press" => state.config.layers[layer_idx].knobs[knob_idx].press = m,
                            "cw" => state.config.layers[layer_idx].knobs[knob_idx].cw = m,
                            _ => {}
                        }
                    }
                    update_yml_preview(&state_clone);
                });

                let config_btn = Button::with_label("⚙");
                let entry_for_btn = entry_clone.clone();
                config_btn.connect_clicked(move |btn| {
                    let window = btn.root().and_downcast::<gtk::Window>().unwrap();
                    let entry_for_callback = entry_for_btn.clone();
                    show_macro_builder(&window, move |m| {
                        entry_for_callback.set_text(&m.to_string());
                    });
                });

                let hbox = Box::new(Orientation::Horizontal, 2);
                hbox.append(&entry_clone);
                hbox.append(&config_btn);
                k_vbox.append(&hbox);
            }
            knob_box.append(&k_vbox);
        }
        
        let layer_box = Box::new(Orientation::Vertical, 12);
        layer_box.append(&grid);
        layer_box.append(&knob_box);

        view_stack.add_titled(&layer_box, Some(&format!("layer_{}", i)), &format!("Layer {}", i));
    }
}

fn char_to_code(c: char) -> Option<Code> {
    match WellKnownCode::from_str(&c.to_lowercase().to_string()) {
        Ok(code) => Some(Code::WellKnown(code)),
        Err(_) if c == ' ' => Some(Code::WellKnown(WellKnownCode::Space)),
        Err(_) => None,
    }
}

fn show_macro_builder<F: Fn(Macro) + 'static>(parent: &gtk::Window, on_ok: F) {
    let dialog = Dialog::builder()
        .title("Macro Builder")
        .transient_for(parent)
        .modal(true)
        .use_header_bar(1)
        .build();

    dialog.add_button("OK", ResponseType::Ok);
    dialog.add_button("Cancel", ResponseType::Cancel);

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let capture_label = Label::new(Some("Press keys to capture or use manual builder below:"));
    content.append(&capture_label);

    let captured_macro = Arc::new(Mutex::new(None::<Macro>));
    let display_label = Label::new(Some("Captured: None"));
    display_label.add_css_class("title-2");
    content.append(&display_label);

    let clear_btn = Button::with_label("Clear Captured");
    let display_label_clear = display_label.clone();
    let captured_macro_clear = captured_macro.clone();
    clear_btn.connect_clicked(move |_| {
        display_label_clear.set_text("Captured: None");
        *captured_macro_clear.lock().unwrap() = None;
    });
    content.append(&clear_btn);

    let key_controller = EventControllerKey::new();
    let captured_macro_clone = captured_macro.clone();
    let display_label_clone = display_label.clone();
    key_controller.connect_key_pressed(move |_, keyval, _keycode, state| {
        let mut modifiers = EnumSet::<Modifier>::empty();
        if state.contains(gtk::gdk::ModifierType::CONTROL_MASK) { modifiers.insert(Modifier::Ctrl); }
        if state.contains(gtk::gdk::ModifierType::SHIFT_MASK) { modifiers.insert(Modifier::Shift); }
        if state.contains(gtk::gdk::ModifierType::ALT_MASK) { modifiers.insert(Modifier::Alt); }
        if state.contains(gtk::gdk::ModifierType::SUPER_MASK) { modifiers.insert(Modifier::Win); }

        let key_name = keyval.name().unwrap_or_default().to_string();
        let well_known = WellKnownCode::from_str(&key_name.to_lowercase());
        let code = match well_known {
            Ok(c) => Some(Code::WellKnown(c)),
            Err(_) => match key_name.to_lowercase().as_str() {
                "return" => Some(Code::WellKnown(WellKnownCode::Enter)),
                "escape" => Some(Code::WellKnown(WellKnownCode::Escape)),
                "backspace" => Some(Code::WellKnown(WellKnownCode::Backspace)),
                "tab" => Some(Code::WellKnown(WellKnownCode::Tab)),
                "space" => Some(Code::WellKnown(WellKnownCode::Space)),
                _ => None,
            }
        };

        if code.is_some() || !modifiers.is_empty() {
            let m = Macro::Keyboard(KeyboardEvent(MacroOptions::default(), vec![Accord::new(modifiers, code)]));
            display_label_clone.set_text(&format!("Captured: {}", m));
            *captured_macro_clone.lock().unwrap() = Some(m);
        }
        gtk::glib::Propagation::Stop
    });
    dialog.add_controller(key_controller);

    content.append(&gtk::Separator::new(Orientation::Horizontal));
    content.append(&Label::new(Some("Manual Builder:")));

    let type_combo = ComboBoxText::new();
    type_combo.append_text("Keyboard");
    type_combo.append_text("Media");
    type_combo.append_text("Mouse");
    type_combo.append_text("Text (max 5 chars)");
    type_combo.append_text("Layer Switch");
    type_combo.set_active(Some(0));
    content.append(&type_combo);

    let stack = gtk::Stack::new();
    
    let kbd_box = Box::new(Orientation::Vertical, 6);
    kbd_box.append(&Label::new(Some("Enter macro text (e.g. ctrl-c):")));
    let kbd_entry = Entry::new();
    kbd_box.append(&kbd_entry);
    stack.add_titled(&kbd_box, Some("keyboard"), "Keyboard");

    let media_combo = ComboBoxText::new();
    for code in [MediaCode::Play, MediaCode::Mute, MediaCode::VolumeUp, MediaCode::VolumeDown, MediaCode::Next, MediaCode::Previous] {
        media_combo.append_text(&code.to_string());
    }
    media_combo.set_active(Some(0));
    stack.add_titled(&media_combo, Some("media"), "Media");

    let mouse_box = Box::new(Orientation::Vertical, 6);
    let mouse_combo = ComboBoxText::new();
    mouse_combo.append_text("Left Click");
    mouse_combo.append_text("Right Click");
    mouse_combo.append_text("Middle Click");
    mouse_combo.append_text("Wheel Up");
    mouse_combo.append_text("Wheel Down");
    mouse_combo.set_active(Some(0));
    mouse_box.append(&mouse_combo);
    stack.add_titled(&mouse_box, Some("mouse"), "Mouse");

    let text_box = Box::new(Orientation::Vertical, 6);
    text_box.append(&Label::new(Some("Enter text to type:")));
    let text_entry = Entry::builder().max_length(5).build();
    text_box.append(&text_entry);
    stack.add_titled(&text_box, Some("text"), "Text");

    let layer_box = Box::new(Orientation::Vertical, 6);
    let layer_combo = ComboBoxText::new();
    layer_combo.append_text("Next Layer");
    for i in 0..16 {
        layer_combo.append_text(&format!("Layer {}", i));
    }
    layer_combo.set_active(Some(0));
    layer_box.append(&layer_combo);
    stack.add_titled(&layer_box, Some("layer"), "Layer Switch");

    content.append(&stack);

    type_combo.connect_changed(move |c| {
        match c.active_text().as_deref() {
            Some("Keyboard") => stack.set_visible_child_name("keyboard"),
            Some("Media") => stack.set_visible_child_name("media"),
            Some("Mouse") => stack.set_visible_child_name("mouse"),
            Some("Text (max 5 chars)") => stack.set_visible_child_name("text"),
            Some("Layer Switch") => stack.set_visible_child_name("layer"),
            _ => {}
        }
    });

    let type_combo_clone = type_combo.clone();
    let kbd_entry_clone = kbd_entry.clone();
    let media_combo_clone = media_combo.clone();
    let mouse_combo_clone = mouse_combo.clone();
    let text_entry_clone = text_entry.clone();
    let layer_combo_clone = layer_combo.clone();

    dialog.connect_response(move |d, response| {
        if response == ResponseType::Ok {
            let final_macro = if let Some(m) = captured_macro.lock().unwrap().clone() {
                Some(m)
            } else {
                match type_combo_clone.active_text().as_deref() {
                    Some("Keyboard") => Macro::from_str(&kbd_entry_clone.text()).ok(),
                    Some("Media") => {
                        let text = media_combo_clone.active_text().unwrap_or_default();
                        MediaCode::from_str(&text).ok().map(Macro::Media)
                    }
                    Some("Mouse") => {
                        match mouse_combo_clone.active_text().as_deref() {
                            Some("Left Click") => Some(Macro::Mouse(MouseEvent(MouseAction::Click(MouseButton::Left.into()), None))),
                            Some("Right Click") => Some(Macro::Mouse(MouseEvent(MouseAction::Click(MouseButton::Right.into()), None))),
                            Some("Middle Click") => Some(Macro::Mouse(MouseEvent(MouseAction::Click(MouseButton::Middle.into()), None))),
                            Some("Wheel Up") => Some(Macro::Mouse(MouseEvent(MouseAction::Wheel(1), None))),
                            Some("Wheel Down") => Some(Macro::Mouse(MouseEvent(MouseAction::Wheel(-1), None))),
                            _ => None
                        }
                    },
                    Some("Text (max 5 chars)") => {
                        let text = text_entry_clone.text();
                        let accords: Vec<Accord> = text.chars()
                            .filter_map(|c| char_to_code(c))
                            .map(|code| Accord::new(EnumSet::empty(), Some(code)))
                            .collect();
                        if !accords.is_empty() {
                            Some(Macro::Keyboard(KeyboardEvent(MacroOptions::default(), accords)))
                        } else {
                            None
                        }
                    },
                    Some("Layer Switch") => {
                        let active = layer_combo_clone.active().unwrap_or(0);
                        if active == 0 {
                            Some(Macro::Layer(0))
                        } else {
                            Some(Macro::Layer((active - 1) as u8))
                        }
                    }
                    _ => None
                }
            };
            if let Some(m) = final_macro {
                on_ok(m);
            }
        }
        d.destroy();
    });

    dialog.present();
}

fn find_keyboard() -> Result<(Device<Context>, DeviceDescriptor, u16)> {
    find_device(&DevelOptions::default())
}

fn set_keyboard_led(mode: u8) -> Result<()> {
    let (_device, _desc, product_id) = find_keyboard()?;
    let mut keyboard = create_driver(product_id, 0, 0)?;
    
    // Construct LED args for k884x or k8890
    let args = if product_id == 0x8890 {
        vec![mode.to_string()]
    } else {
        // Default to layer 0 and the provided mode stringified if possible, 
        // but GUI currently simplified to just mode index.
        vec!["0".to_string(), format!("backlight white")] // Placeholder for 884x
    };

    let (handle, endpoint, _) = ch57x_keyboard_tool::open_device(&DevelOptions::default())?;
    let mut output = Vec::new();
    keyboard.set_led(&args, &mut output)?;
    ch57x_keyboard_tool::send_to_device(&handle, endpoint, &output)?;
    Ok(())
}

fn upload_config(config: &Config) -> Result<()> {
    let (_device, _desc, product_id) = find_keyboard()?;
    let (buttons, knobs) = (config.rows * config.columns, config.knobs);
    let keyboard = create_driver(product_id, buttons, knobs)?;

    let layers = config.clone().render()?;
    let empty_macro = Macro::Keyboard(KeyboardEvent(MacroOptions::default(), vec![]));

    let (handle, endpoint, _) = ch57x_keyboard_tool::open_device(&DevelOptions::default())?;
    let mut output = Vec::new();

    for (layer_idx, layer) in layers.iter().enumerate() {
        for (button_idx, macro_) in layer.buttons.iter().enumerate() {
            let m = macro_.as_ref().unwrap_or(&empty_macro);
            keyboard.bind_key(layer_idx as u8, Key::Button(button_idx as u8), m, &mut output)?;
        }

        for (knob_idx, knob) in layer.knobs.iter().enumerate() {
            let ccw = knob.ccw.as_ref().unwrap_or(&empty_macro);
            keyboard.bind_key(layer_idx as u8, Key::Knob(knob_idx as u8, KnobAction::RotateCCW), ccw, &mut output)?;

            let press = knob.press.as_ref().unwrap_or(&empty_macro);
            keyboard.bind_key(layer_idx as u8, Key::Knob(knob_idx as u8, KnobAction::Press), press, &mut output)?;

            let cw = knob.cw.as_ref().unwrap_or(&empty_macro);
            keyboard.bind_key(layer_idx as u8, Key::Knob(knob_idx as u8, KnobAction::RotateCW), cw, &mut output)?;
        }
    }

    ch57x_keyboard_tool::send_to_device(&handle, endpoint, &output)?;
    Ok(())
}