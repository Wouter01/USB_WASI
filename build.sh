set -e

# cargo component update
# cargo component build

echo building guest...
cargo component build -p usb-component-wasi-guest

echo building stadia...
cargo component build -p usb-component-wasi-stadia

echo building host...
cargo build -p usb_wasi_host
