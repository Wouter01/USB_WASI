set -e

echo "Running mass storage (WASI)..."

cargo component build --release -p usb-mass-storage
cargo build --release -p usb-wasi-host

echo "To access certain USB devices, sudo may be required"

./target/release/usb-wasi-host --usb-use-denylist target/wasm32-wasi/release/usb-mass-storage.wasm
