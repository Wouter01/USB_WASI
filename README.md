# USB_WASI

A prototype for adding USB support to WASI.
The repository contains an initial WIT interface, alongside a host implementation and guest example usage.

In a later stage, the WIT interface in this repo will be merged with [@Wadu436](https://github.com/Wadu436)'s [one](https://github.com/Wadu436/usb-wasm) as part of the creation of a [proposal](https://github.com/Wadu436/wasi-usb) for standardization.

# Running the example
## Install
Add the WASI target:
```
rustup target add wasm32-wasi
```
```
cargo install wasm-tools
```
This repo uses cargo component to easily build WASM binaries with the component model:
```
cargo install cargo-component
```

## Running the host
Use `cargo run` to build and run the host:
```
cargo run --release -p usb_wasi_host -- --usb-devices 1234:1234 path_to_guest.wasm
```

The host supports the following parameters:
```
Usage: usb-wasi-host [OPTIONS] <COMPONENT_PATH>

Arguments:
  <COMPONENT_PATH>  The path to the guest component

Options:
      --usb-devices <USB_DEVICES>  Comma-separated list of USB devices to allow (in hex format: vendor_id:product_id, e.g. 12AB:34CD)
      --usb-use-denylist           Use a denylist for USB devices instead of an allowlist
  -h, --help                       Print help
  -V, --version                    Print version
```

## Running the examples
For each example a .sh file is included which will compile the example code and run it. `cargo component` is used to build the wasm files in the script. If there are errors because Wasmtime could not link the WIT file correctly, you may need to run
```
cargo component update
```

Available examples:
- Receiving data from Arduino (native, WASI)
- Reading mass storage file tree and contents (native, WASI)
- Reading and writing state to Stadia controller (WASI)

Note that some of the examples might require `sudo` to correctly release a kernel interface. (If not given but required, the code will panic with "insufficient permissions")
