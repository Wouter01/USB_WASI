#[cfg(target_arch = "wasm32")]
mod bindings;

#[cfg(target_arch = "wasm32")]
use bindings::component::usb::{
    types::{Direction, TransferType},
    descriptors::EndpointDescriptor,
    usb::{UsbDevice, DeviceHandle}
};

#[cfg(not(target_arch = "wasm32"))]
use rusb::{DeviceHandle, request_type};

use std::{fs::File, thread::sleep, time::{Duration, Instant}};
use std::io::Write;

use anyhow::{anyhow, Result};

#[cfg(not(target_arch = "wasm32"))]
const DURATION: Duration = Duration::from_secs(10);

#[cfg(target_arch = "wasm32")]
const DURATION: u64 = 10_000_000_000; // 10 seconds

#[cfg(target_arch = "wasm32")]
fn connect_to_arduino() -> Result<(DeviceHandle, u16, u8)> {

    let devices = UsbDevice::enumerate();

    let device = devices
        .iter()
        .find(|d| {
        let props = d.device_descriptor();
        props.product_id == 0x8037 && props.vendor_id == 0x2341
        })
        .ok_or(anyhow!("Could not find Arduino Micro."))?;

    let configurations = device.configurations()?;

    let configuration = configurations
        .first()
        .ok_or(anyhow!("Device has no configurations"))?;

    let interface_descriptor = configuration
        .interfaces
        .iter()
        .find(|i| i.class_code == 0xff)
        .ok_or(anyhow!("Device has no device-specific class code."))?;

    let endpoint_in = interface_descriptor
        .endpoint_descriptors
        .iter()
        .find(|e| e.direction == Direction::In && e.transfer_type == TransferType::Bulk)
        .ok_or(anyhow!("No endpoint in interface with direction IN and transfer type Interrupt"))?;

    let handle = device.open()?;

    handle.select_configuration(configuration.number);
    for interface in configuration.interfaces.to_owned() {
        handle.claim_interface(interface.number);
    }

    let buffer: [u8; 10] = [0x01; 10];
    // Send DTR signal.
    let _ = handle.write_control(33, 0x22, 0x01, interface_descriptor.number as u16, &buffer, DURATION);

    Ok((handle, endpoint_in.max_packet_size, endpoint_in.address))
}

#[cfg(not(target_arch = "wasm32"))]
fn connect_to_arduino() -> Result<(DeviceHandle<rusb::Context>, u16, u8)> {
    use rusb::{Direction, Recipient, RequestType, TransferType, UsbContext};


    let context = rusb::Context::new();

    let device = context?
        .devices()?
        .iter()
        .find(|d| {
            let Ok(descriptor) = d.device_descriptor() else { return false };
            descriptor.product_id() == 0x8037 && descriptor.vendor_id() == 0x2341
        })
        .ok_or(anyhow!("Could not find Arduino device"))?;

    let configuration = device.config_descriptor(0)?;

    let interface = configuration
        .interfaces()
        .find(|i| i.descriptors().next().unwrap().class_code() == 0xff)
        .ok_or(anyhow!("Interface not found"))?;

    let descriptor = interface
        .descriptors()
        .find(|_| true)
        .ok_or(anyhow!("Descriptor not found"))?;

    let endpoint = descriptor
        .endpoint_descriptors()
        .find(|e| e.direction() == Direction::In && e.transfer_type() == TransferType::Bulk)
        .ok_or(anyhow!("No endpoint in interface with direction IN and transfer type Interrupt"))?;

    let mut handle = device.open()?;
    handle.set_auto_detach_kernel_driver(true)?;
    handle.set_active_configuration(configuration.number())?;
    for i in 0..3 {
        handle.detach_kernel_driver(i)?;
        handle.claim_interface(i)?;
    }

    let request = request_type(Direction::Out, RequestType::Class, Recipient::Interface);
    let buffer: [u8; 10] = [0x01; 10];
    // Send DTR signal.
    let _ = handle.write_control(request, 0x22, 0x01, interface.number() as u16, &buffer, DURATION);

    Ok((handle, endpoint.max_packet_size(), endpoint.address()))
}


fn main() -> Result<()> {
    let (handle, max_packet_size, endpoint_address) = connect_to_arduino()?;

    println!("Connected to controller");

    let mut buffer: Vec<u8> = vec![0; max_packet_size as usize];

    let mut times: Vec<Duration> = vec![];

    const RUNS: u8 = 5;
    let mut data: Vec<Duration> = Vec::with_capacity(RUNS as usize * 10000 as usize);

    for i in 0..RUNS {
        println!("Run {}/{}", i+1, RUNS);
        let now = Instant::now();
        for _ in 0..10000 {
            let one_measure = Instant::now();

            #[cfg(target_arch = "wasm32")]
            let _ = handle.read_bulk(endpoint_address, max_packet_size as u64, DURATION)?;

            #[cfg(not(target_arch = "wasm32"))]
            let _ = handle.read_bulk(endpoint_address, &mut buffer, DURATION)?;

            let elapsed = one_measure.elapsed();

            data.push(elapsed);
            sleep(Duration::from_millis(2));
        }
        times.push(now.elapsed());
    }

    println!("Done. {:?}", times);

    // Open a file in write mode
    let mut file = File::create("durations_wasi.txt")?;

    // Write each duration to the file
    for duration in &data {
        writeln!(file, "{}", duration.as_micros())?;
    }

    println!("Durations have been written to the file.");

    Ok(())
}
