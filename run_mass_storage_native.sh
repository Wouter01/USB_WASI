set -e

echo "Running mass storage (native)..."

cargo build --release -p usb-mass-storage

echo "To access certain USB devices, sudo may be required"

./target/release/usb-mass-storage
