set -e

echo "Running Arduino (native)..."

cargo build --release -p usb-arduino

echo "To access an Arduino device, sudo may be required"

./target/release/usb-arduino
