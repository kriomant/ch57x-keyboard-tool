# What is this?

This is an utility for programming small keyboards like this one:

![](doc/keyboard-12-2.png)

or this one:

![](doc/keyboard-3-1.jpg)

and other...


There are several modifications of such keyboards with different number of
buttons and knobs. Utility was tested to work with:
 * 3×3 with 2 knobs
 * 3×4 with 2 knobs (Bluetooth version)
 * 3x2 with 1 knob 
 * 1x3 with 1 knob
 
Such keyboards are popular on AliExpress and seller usually sends software
for programming, but it:
 * requires Windows,
 * is very ugly and inconvenient,
 * can only program one key at a time
 * don't expose all keyboard features

# How to use?

Sorry, right now there are no prebuilt binaries, may be fixed later.
Install *cargo* utility using [rustup](https://rustup.rs/), then execute
`cargo install ch57x-keyboard-tool`.

Now create you own config from provided *example-mapping.yaml*, and apply:

    ch57x-keyboard-tool upload < your-config.yaml

You can also change LED configuration, if you keyboard supports it:

    ch57x-keyboard-tool led 1

# Diagnostics

If you have any troubles using this software, please provide diagnostics.

## Getting list of attached USB devices

### MacOS


    ioreg -w0 -l -p IOUSB

or

    system_profiler SPUSBDataType
    
### Linux


    lsusb -v

## Monitoring generated keyboard and mouse events

Most simple (and cross-platform) way I've found is using `keyboard` and `mouse` Python modules.

Monitoring keyboard:

    pip3 install keyboard
    sudo python3 -m keyboard

Monitoring mouse:

    # Latest published 'mouse' module doesn't support MacOS, so use latest version from Git:
    git clone https://github.com/boppreh/mouse
    cd mouse
    python3 -m mouse
