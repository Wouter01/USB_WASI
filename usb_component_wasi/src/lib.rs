cargo_component_bindings::generate!();


use crate::bindings::exports::component::usb::device::GuestDevice;

pub struct Device {
    
}


impl GuestDevice for Device {
    fn test(&self) -> String {
        "boe".to_string()
    }
    
    fn new(list: Vec<u8>) -> Self {
        Self { }
    }
}

struct Component;