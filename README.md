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
