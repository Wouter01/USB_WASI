cargo_component_bindings::generate!();

use crate::bindings::component::usb::{device::get_devices, events::{update, DeviceConnectionEvent}};
use crate::bindings::Guest;

use crate::bindings::component::usb::device::UsbDevice;

struct Component;

fn main() {}

impl Guest for Component {
    #[tokio::main(flavor = "current_thread")]
    async fn run() {
        let all_devices: Vec<UsbDevice> = get_devices();

        let mapped = all_devices
            .iter()
            .map(|d| {
                (
                    d.get_name(), 
                    d.configurations()
                    .iter()
                    .map(|c| c.name.clone().unwrap_or("?".to_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
                )
            })
            .collect::<Vec<_>>();

        println!("Device names: {:#?}", mapped);
        
        tokio::spawn(async {
            loop {
                match update() {
                    DeviceConnectionEvent::Connected(device) => {
                        let name = device.get_name();
                        println!("Connected: {:?}", name);
                    },
                    DeviceConnectionEvent::Disconnected(device) => {
                        let name = device.properties().product_id;
                        println!("Disconnected: {:?}", name);
                    },

                    DeviceConnectionEvent::Pending => tokio::time::sleep(std::time::Duration::from_secs(1)).await,
                    DeviceConnectionEvent::Closed => println!("Closed Connection.")
                }
            }
        });
        
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        println!("Stopped sleeping...");
    }
}