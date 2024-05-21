set -e

# cargo component update
# cargo component build

echo building mass storage...
cargo component build --release -p usb-mass-storage-native

echo building stadia...
cargo component build -p usb-component-wasi-stadia

echo building arduino...
cargo component build --release -p usb-component-wasi-arduino

echo building host...
cargo build --release -p usb_wasi_host
