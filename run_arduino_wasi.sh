set -e

echo "Running Arduino benchmark (WASI)..."

cargo component build --release -p usb-arduino
cargo build --release -p usb-wasi-host

echo "To access an Arduino device, sudo may be required"

./target/release/usb-wasi-host --usb-use-denylist target/wasm32-wasi/release/usb-arduino.wasm
