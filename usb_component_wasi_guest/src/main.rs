cargo_component_bindings::generate!();

use crate::bindings::component::usb::device::get_devices;
use crate::bindings::Guest;
// use crate::bindings::usb_device::Device as Device;

fn main() {
    // let device = UsbDevice::new(&test);
    // let output = device.test();
    let all_devices = get_devices();
    
    println!("Devices: {:?}", all_devices);
    // println!("{:?}", output);
    println!("Hello, worlddd!");
}

struct Component;

impl Guest for Component {
    fn hello() {
        // let device = UsbDevice::new(&test);
        // let output = device.test();
        let all_devices = get_devices();
        let devices = all_devices
        .iter()
        .map(|d| (d.properties(), d.configurations()))
        .collect::<Vec<_>>();
        
        println!("Devices: {:#?}", devices);
    }
}