cargo_component_bindings::generate!();

use crate::bindings::component::usb::device::{get_devices};
use crate::bindings::component::usb::types::{Configuration, Properties};
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
    fn hello() -> u32 {
        // let device = UsbDevice::new(&test);
        // let output = device.test();
        let all_devices = get_devices();
        let props: Vec<Properties> = all_devices.iter().map(|d| d.properties()).collect();
        
        let configs: Vec<Vec<Configuration>> = all_devices.iter().map(|d| d.configurations()).collect();
        
        let devices = std::iter::zip(props, configs).collect::<Vec<_>>();
        println!("Devices: {:#?}", devices);
        
        // println!("{:?}", output);
        println!("Hello, worldddd!");
        5
    }
}