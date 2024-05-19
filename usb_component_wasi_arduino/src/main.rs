mod bindings;

use std::{fs::File, thread::sleep, time::{Duration, Instant}};
use std::io::Write;

use anyhow::{anyhow, Result};
use bindings::component::usb::types::{Direction, TransferType};

use crate::bindings::component::usb::usb::UsbDevice;

fn main() -> Result<()> {
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

    println!("{:#?}", configuration.interfaces);

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

    let endpoint_out = interface_descriptor
        .endpoint_descriptors
        .iter()
        .find(|e| e.direction == Direction::Out && e.transfer_type == TransferType::Bulk)
        .ok_or(anyhow!("No endpoint in interface with direction OUT and transfer type Interrupt"))?;

    let handle = device.open()?;

    handle.select_configuration(configuration.number);
    for interface in configuration.interfaces.to_owned() {
        handle.claim_interface(interface.number);
    }

    let buffer: [u8; 10] = [0x01; 10];
    handle.write_control(33, 0x22, 0x01, interface_descriptor.number as u16, &buffer)
        .map_err(|e| e.to_string());

    println!("Connected to controller");

    let max_packet_size = endpoint_in.max_packet_size;
    dbg!(max_packet_size);
    let mut buffer: Vec<u8> = vec![0; max_packet_size as usize];

    let mut times: Vec<Duration> = vec![];

    // Warm up
    println!("Warming up...");
    // for _ in 0..10000 {
    //     let (bytes_written, data) = handle.read_bulk(endpoint_in.address, max_packet_size)
    //         .map_err(|e| e.to_string())?;
    // }

    const RUNS: u8 = 5;
    let mut data: Vec<Duration> = Vec::with_capacity(RUNS as usize * 10000 as usize);

    for i in 0..RUNS {
        println!("Run {}/{}", i+1, RUNS);
        let now = Instant::now();
        for _ in 0..10000 {
            let one_measure = Instant::now();
            let (bytes_written, data2) = handle.read_bulk(endpoint_in.address, max_packet_size)?;
            let elapsed = one_measure.elapsed();
            let cutted = &data2[0..(bytes_written as _)];
            let str = String::from_utf8(cutted.to_vec());

            // println!("{} {:?} {:?}", bytes_written, elapsed, 0);
            // dbg!(bytes_written, elapsed);
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

    // let now = Instant::now();

    // for i in 0..10000 {
    //     let (bytes_written, data) = handle
    //     .read_bulk(endpoint_in.address)
    //     .map_err(|e| e.to_string())?;
    //     // .map_err(|e| e.to_string())?;

    //     if i % 1000 == 0 {
    //         println!("{}", i);
    //     }
    //     // println!("Bytes written {}: {}", i, bytes_written);
    // }

    // let elapsed_time = now.elapsed();
    // println!("Reading state took {} milliseconds.", elapsed_time.as_millis());

    // let (bytes_received, data) = handle
    //     .read_bulk(endpoint_in.address)
    //     .map_err(|e| e.to_string())?;

    // println!("Received bytes: {:?}", data);

    Ok(())
}
