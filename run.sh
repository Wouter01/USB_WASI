# on macOS, sudo is required to be able to detach kernel extensions
sudo cargo run -p usb_wasi_host target/wasm32-wasi/debug/usb-component-wasi-guest.wasm
