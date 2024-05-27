set -e

echo "Running Stadia benchmark (WASI)..."

cargo component build --release -p usb-stadia
cargo build --release -p usb-wasi-host

echo "To access certain USB devices, sudo may be required"

./target/release/usb-wasi-host --usb-devices 18d1:9400 target/wasm32-wasi/release/usb-stadia.wasm
