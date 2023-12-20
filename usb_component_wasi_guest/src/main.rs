cargo_component_bindings::generate!();

use crate::bindings::component::usb::device::get_devices;
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
                    d.properties().device_name, 
                    d.configurations()
                    .iter()
                    .map(|c| c.name.clone().unwrap_or("?".to_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
                )
            })
            .collect::<Vec<_>>();

        println!("Device names: {:#?}", mapped);
        
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}