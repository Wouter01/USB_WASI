mod bindings;

use crate::bindings::{
    Guest,
    component::usb::{
        device::{get_devices, UsbDevice}, 
        events::{update, DeviceConnectionEvent}
    }
};

use tokio::time::{sleep, Duration};

struct Component;

impl Component {
    fn get_device_config_names(device: &UsbDevice) -> Option<String> {
        let configs = device.configurations();
        match configs {
            Err(_) => None,
            Ok(configs) => {   
                Some(configs
                .iter()
                .map(|c| c.name.clone().unwrap_or("?".to_string()))
                .collect::<Vec<_>>()
                .join(", "))
            }
        }
    }
    
    fn get_all_device_names() -> Vec<(String, Option<String>)> {
        let all_devices: Vec<UsbDevice> = get_devices();
        
        let mapped = all_devices
            .iter()
            .map(|d| {
                (
                    d.get_name().unwrap_or("Could not resolve name".to_string()), 
                    Component::get_device_config_names(d)
                )
            })
            .collect::<Vec<_>>();
            
        mapped
    }
}

impl Guest for Component {
    #[tokio::main(flavor = "current_thread")]
    async fn run() {
        println!("Device names: {:#?}", Component::get_all_device_names());
        
        tokio::spawn(async {
            loop {
                match update() {
                    DeviceConnectionEvent::Connected(device) => {
                        let name = device.get_name();
                        println!("Connected: {:?}", name);
                        println!("Configurations: {:?}", Self::get_device_config_names(&device));
                    },
                    
                    DeviceConnectionEvent::Disconnected(device) => {
                        let product_id = device.properties().product_id;
                        println!("Disconnected: {:?}", product_id);
                    },

                    DeviceConnectionEvent::Pending => sleep(Duration::from_secs(1)).await,
                    
                    DeviceConnectionEvent::Closed => {
                        println!("Closed Connection.");
                        break;
                    }
                }
            }
            
        });
        
        sleep(std::time::Duration::from_secs(120)).await;
    }
}

fn main() {}