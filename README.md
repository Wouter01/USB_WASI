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

## Build
Building the guest:
```
cargo component build -p usb-component-wasi-guest
```
> Note: Updating the component dependencies may be necessary
> ```
> cargo component update
> ```


Building the host:
```
cargo build -p usb_wasi_host
```

## Run
The host program requires the location of the WASM binary as a parameter (for now):
```
cargo run -p usb_wasi_host target/wasm32-wasi/debug/usb-component-wasi-guest.wasm
```

The repo also contains small scripts (clean.sh, build.sh, run.sh) which do these commands.
