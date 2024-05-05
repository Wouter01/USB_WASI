use std::{fs::File, num::Wrapping, process::exit, time::{Duration, Instant}};
use std::io::{self, Write};

use anyhow::{anyhow, Result};
use bitflags::bitflags;
use tokio::{task::AbortHandle, time::sleep};

use rusb::*;

// fn read_property<S>(&self, perform: impl FnOnce(&rusb::Device<T>, DeviceHandle<T>, &Language) -> Result<S, rusb::Error>) -> Result<S, DeviceHandleError> {
//     let device = &self.device;
//     let handle = device.open().map_err(DeviceHandleError::from)?;
//     let languages = handle.read_languages(DEFAULT_TIMEOUT).map_err(DeviceHandleError::from)?;
//     let language = languages
//         .first()
//         .ok_or(DeviceHandleError::Other)?;

//     perform(device, handle, language)
//         .map_err(DeviceHandleError::from)
// }

fn main() -> anyhow::Result<()> {
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

    for interface in configuration.interfaces().map(|i| i.descriptors()).collect::<Vec<_>>() {
        for descriptor in interface {
            for endpoint in descriptor.endpoint_descriptors() {
                println!("{:?} {:?}", endpoint.direction(), endpoint.transfer_type())
            }
            // println!("{:#?}", descriptor.endpoint_descriptors().collect::<Vec<_>>());
        }
    }

    let interface = configuration
        .interfaces()
        .find(|i| i.descriptors().next().unwrap().class_code() == 0xff)
        // .find(|i| i.number() == 1)
        .ok_or(anyhow!("Interface not found"))?;

    let descriptor = interface
        .descriptors()
        .find(|_| true)
        .ok_or(anyhow!("Descriptor not found"))?;

    let endpoint = descriptor
        .endpoint_descriptors()
        .find(|e| e.direction() == Direction::In && e.transfer_type() == TransferType::Bulk)
        .ok_or(anyhow!("No endpoint in interface with direction IN and transfer type Interrupt"))?;

    // let endpoint_out = descriptor
    //     .endpoint_descriptors()
    //     .find(|e| e.direction() == Direction::Out && e.transfer_type() == TransferType::Bulk)
    //     .ok_or(anyhow!("No endpoint in interface with direction IN and transfer type Interrupt"))?;

    let mut handle = device.open()?;
    handle.set_auto_detach_kernel_driver(true)?;
    handle.set_active_configuration(configuration.number())?;
    for i in 0..3 {
        handle.detach_kernel_driver(i)?;
        handle.claim_interface(i)?;
    }
    // handle.claim_interface(descriptor.interface_number())?;
    // handle.set_alternate_setting(descriptor.interface_number(), 0)?;

    const DURATION: Duration = Duration::from_secs(10);

    let request = request_type(Direction::Out, RequestType::Class, Recipient::Interface);

    println!("Request {}", request);
    let buffer: [u8; 10] = [0x01; 10];
    handle.write_control(request, 0x22, 0x01, interface.number() as u16, &buffer, Duration::from_secs(1));
    let max_packet_size = endpoint.max_packet_size();

    let mut buffer: Vec<u8> = vec![0; max_packet_size as usize];

    let mut times: Vec<Duration> = vec![];

    // Warm up
    println!("Warming up...");
    for _ in 0..10000 {
        handle.read_bulk(endpoint.address(), &mut buffer, DURATION)?;
    }

    const RUNS: u8 = 100;
    let mut data: Vec<Duration> = Vec::with_capacity(RUNS as usize * 10000 as usize);

    for i in 0..RUNS {
        println!("Run {}/{}", i+1, RUNS);
        let now = Instant::now();
        for _ in 0..10000 {
            let one_measure = Instant::now();
            handle.read_bulk(endpoint.address(), &mut buffer, DURATION)?;
            data.push(one_measure.elapsed());
        }
        times.push(now.elapsed());
    }

    println!("Done. {:?}", times);
    println!("Runs size: {}", data.len());

    // Open a file in write mode
    let mut file = File::create("durations.txt")?;

    // Write each duration to the file
    for duration in &data {
        writeln!(file, "{}", duration.as_micros())?;
    }

    println!("Durations have been written to the file.");

    Ok(())
}
