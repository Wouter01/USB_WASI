mod bindings;

use anyhow::Result;
use bindings::component::usb::types::{Direction, TransferType};

use crate::bindings::{
    component::usb::{
        device::{get_devices, UsbDevice},
        events::{update, DeviceConnectionEvent},
    },
    Guest,
};

use tokio::time::{sleep, Duration};

bindings::export!(Component with_types_in bindings);

struct Component;

impl Component {
    fn get_device_config_names(device: &UsbDevice) -> Option<String> {
        let configs = device.configurations();
        match configs {
            Err(_) => None,
            Ok(configs) => Some(
                configs
                    .iter()
                    .map(|c| c.name.clone().unwrap_or("?".to_string()))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        }
    }

    fn get_all_device_names() -> Vec<(String, Option<String>)> {
        let all_devices: Vec<UsbDevice> = get_devices();

        let mapped = all_devices
            .iter()
            .map(|d| {
                (
                    d.product_name().unwrap_or("Could not resolve name".to_string()),
                    Component::get_device_config_names(d),
                )
            })
            .collect::<Vec<_>>();

        mapped
    }

    fn send_data(device: &UsbDevice) -> Result<()> {
        let configuration = device.configurations()?.into_iter().find_map(|c| {
            let descriptors = c
                .interfaces
                .into_iter()
                .map(|i| (i.number, i.descriptors))
                .find_map(|(number, v)| {
                    let interface = v.into_iter().find_map(|d| {
                        let endpoint = d.endpoint_descriptors.into_iter().find(|e| {
                            e.direction == Direction::Out && e.transfer_type == TransferType::Bulk
                        });

                        endpoint
                    });

                    interface.map(|d| (number, d))
                });

            descriptors.map((|(n, interface)| (c.number, n, interface)))
        });

        let configuration2 = device.configurations()?.into_iter().find_map(|c| {
            let descriptors = c
                .interfaces
                .into_iter()
                .map(|i| (i.number, i.descriptors))
                .find_map(|(number, v)| {
                    let interface = v.into_iter().find_map(|d| {
                        let endpoint = d.endpoint_descriptors.into_iter().find(|e| {
                            e.direction == Direction::In && e.transfer_type == TransferType::Interrupt
                        });

                        endpoint
                    });

                    interface.map(|d| (number, d))
                });

            descriptors.map((|(n, interface)| (c.number, n, interface)))
        });

        println!("{:#?}", configuration);
        if let Some((configuration_number, i_number, endpoint)) = configuration {
            println!("Opening device.");
            let handle = device.open()?;
            // handle.set_configuration(configuration_number);
            // handle.claim_interface(i_number);
            // println!(
            //     "Using numbers: {:?}, {:?}, {:?}",
            //     configuration_number, i_number, endpoint.number
            // );
            // let result = handle.write_bulk(
            //     endpoint.number,
            //     &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
            // )?;
            // println!("Wrote data {:?}", result);
            // handle.unclaim_interface(i_number);

            if let Some((configuration_number, i_number, endpoint)) = configuration2 {
                let has_kernel_driver = match handle.kernel_driver_active(1) {
                    Ok(true) => {
                        handle.detach_kernel_driver(1).ok();
                        true
                    }
                    _ => false,
                };
                println!("Has kernel driver? {:?}", has_kernel_driver);
                handle.select_configuration(1);
                handle.claim_interface(1);
                handle.select_alternate_interface(1, 0)?;

                println!(
                    "Config: {:?}, Interface: {:?}, Endpoint: {:?}, Endpoint Address: {:?}",
                    configuration_number, i_number, endpoint.number, endpoint.address
                );
                let data: Vec<u8> = vec![1,2,3];
                let bytes_written = handle.write_interrupt(131, data.as_slice())?;
                println!("Wrote data {:?}", bytes_written);

                let answer = handle.read_interrupt(131);
                println!("Read data {:?}", answer);

            }
        }

        Ok(())
        // match handle {

        //     Err(e) => println!("Error opening device: {:?}", e),
        //     Ok(handle) => {
        //         println!("Opened device.");

        //         handle.claim_interface(0);
        //         handle.set_configuration(1);
        //         // handle.write_interrupt_out(endpoint, data)

        //     }
        // }
    }
}

impl Guest for Component {
    #[tokio::main(flavor = "current_thread")]
    async fn run() -> Result<(), String> {
        println!("Device names: {:#?}", Component::get_all_device_names());

        tokio::spawn(async {
            loop {
                match update() {
                    DeviceConnectionEvent::Connected(device) => {
                        let name = device.product_name();
                        println!("Connected: {:?}", name);
                        // println!("Configurations: {:?}", Self::get_device_config_names(&device));
                        if let Ok(name) = name {
                            if name.contains("Arduino") {
                                println!("{:#?}", device.configurations());
                                loop {
                                    match Self::send_data(&device) {
                                        Err(e) => println!("Error sending data: {:?}", e),
                                        Ok(_) => println!("Data sent."),
                                    }
                                    sleep(Duration::from_secs(1)).await;
                                }
                                // handle.unwrap().set_configuration(configuration);

                                // let result = transfer_interrupt_in(&device);
                            }
                        }
                    }

                    DeviceConnectionEvent::Disconnected(device) => {
                        let product_id = device.properties().product_id;
                        println!("Disconnected: {:?}", product_id);
                    }

                    DeviceConnectionEvent::Pending => sleep(Duration::from_secs(1)).await,

                    DeviceConnectionEvent::Closed => {
                        println!("Closed Connection.");
                        break;
                    }
                }
            }
        });

        sleep(std::time::Duration::from_secs(120)).await;

        Ok(())
    }
}

fn main() {
    println!("Starting guest.");
}
