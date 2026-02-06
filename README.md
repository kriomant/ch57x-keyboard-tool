# ch57x-keyboard-tool

This is a utility for programming small CH57x-based macro keyboards.

## Features
- **GUI Tool**: A modern GTK4/Adwaita interface for easy configuration.
- **CLI Tool**: Scriptable command-line interface for uploading mappings.
- **Multi-Layer Support**: Configure up to 16 layers (hardware permitting).
- **Macro Builder**: Live hotkey capture and manual macro building.
- **Diagnostic Console**: Built-in hardware detection and permission fixing.

## Installation

### Prerequisites (Linux)
You need GTK4, Libadwaita, and USB development libraries:
```bash
# Debian/Ubuntu
sudo apt install pkg-config libgtk-4-dev libadwaita-1-dev libusb-1.0-0-dev
```

### Build
```bash
cargo build --release
```

## Usage

### GUI Tool (Recommended)
Launch the graphical interface to configure macros, layers, and hardware settings visually:
```bash
cargo run --bin ch57x-keyboard-gui
```

### CLI Tool
Upload a mapping from a YAML file via stdin:
```bash
ch57x-keyboard-tool upload < your-config.yaml
```
Select LED backlight mode:
```bash
ch57x-keyboard-tool led 1
```

## Diagnostics
The GUI tool includes a **Diagnostic Console** at the bottom. If you see "Permissions Denied", click the **"Fix Linux Permissions"** button in the app to automatically install the required udev rules.
