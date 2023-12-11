cargo_component_bindings::generate!();

pub mod device;

use crate::bindings::exports::component::usb::device::GuestUsbDevice;
pub use crate::device::UsbDevice;

impl GuestUsbDevice for UsbDevice {
    fn test(&self) -> String {
        "boe".to_string()
    }
    
    fn new(list: Vec<u8>) -> Self {
        Self { }
    }
}

struct Component;