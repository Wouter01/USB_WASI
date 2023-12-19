cargo_component_bindings::generate!();

use crate::bindings::component::usb::device::get_devices;
use crate::bindings::Guest;
use crate::bindings::exports::component::usb::events::Guest as EventsGuest;
use crate::bindings::component::usb::device::UsbDevice;

fn main() {}

struct Component;


impl Guest for Component {
    #[tokio::main(flavor = "current_thread")]
    async fn hello() {
        let all_devices = get_devices();
        let devices = all_devices
        .iter()
        .map(|d| (d.properties(), d.configurations()))
        .collect::<Vec<_>>();
        
        println!("Devices: {:#?}", devices);
    }
}

impl EventsGuest for Component {
    fn device_added(_: UsbDevice) {
        println!("Added Device.");
    }
    
    fn device_removed(_: UsbDevice) {
        println!("Added Device.");
    }
}