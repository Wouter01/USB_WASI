cargo_component_bindings::generate!();

use crate::bindings::component::usb::device::Device;

// use crate::bindings::usb_device::Device as Device;

fn main() {
    let test: [u8; 1] = [1];
    let device = Device::new(&test);
    let output = device.test();
    
    println!("{:?}", output);
    println!("Hello, world!");
}
