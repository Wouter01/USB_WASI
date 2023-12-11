cargo_component_bindings::generate!();

use crate::bindings::component::usb::device::UsbDevice;
use crate::bindings::component::usb::device::*;
use crate::bindings::Guest;
// use crate::bindings::usb_device::Device as Device;

fn main() {
    let test: [u8; 1] = [1];
    // let device = UsbDevice::new(&test);
    // let output = device.test();
    let all_devices = get_devices();
    
    println!("Devices: {:?}", all_devices);
    // println!("{:?}", output);
    println!("Hello, worlddd!");
}

struct Component;

impl Guest for Component {
    fn hello() -> u32 {
        let test: [u8; 1] = [1];
        // let device = UsbDevice::new(&test);
        // let output = device.test();
        let all_devices = get_devices();
        let props = all_devices[0].properties();
        println!("Devices: {:?}", all_devices);
        // println!("{:?}", output);
        println!("Hello, worldddd!");
        5
    }
}