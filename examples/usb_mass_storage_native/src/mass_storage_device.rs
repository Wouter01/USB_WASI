use std::{io::{Read, Seek}, time::Duration};
use anyhow::{anyhow, Result};
use rusb::{request_type, Context, DeviceHandle, Direction, TransferType, UsbContext};

pub struct MassStorageDevice {
    device_handle: DeviceHandle<Context>,
    interface_number: u8,
    seek_position: u64
}

impl MassStorageDevice {
    pub fn new() -> Result<Self> {
        let context = rusb::Context::new().unwrap();
        let device = context
            .devices()?
            .iter()
            .find(|d| {
                let config = d.config_descriptor(0).unwrap();
                config
                    .interfaces()
                    // 0x08 = Mass storage class code
                    // 0x50 = Bulk only transport
                    .find(|i| i.descriptors().find(|i| i.class_code() == 0x08 && i.protocol_code() == 0x50).is_some())
                    .is_some()
            })
            .ok_or(anyhow!("No mass storage device found."))?;

        let configuration = device.config_descriptor(0).unwrap();
        let interface = configuration
            .interfaces()
            .find_map(|i| i.descriptors().find(|i| i.class_code() == 0x08 && i.protocol_code() == 0x50))
            .ok_or(anyhow!("No mass storage interface found."))?;

        let endpoint_in = interface
            .endpoint_descriptors()
            .find(|e| e.transfer_type() == TransferType::Bulk && e.direction() == Direction::In)
            .ok_or(anyhow!("No Bulk In Endpoint found,"))?;

        let endpoint_out = interface
            .endpoint_descriptors()
            .find(|e| e.transfer_type() == TransferType::Bulk && e.direction() == Direction::Out)
            .ok_or(anyhow!("No Bulk Out Endpoint found,"))?;

        let mut handle = device.open()?;
        handle.reset()?;
        handle.set_auto_detach_kernel_driver(true)?;
        handle.set_active_configuration(0)?;

        // Claim interface with bulk endpoints
        handle.claim_interface(interface.interface_number())?;

        // Claim interface with control endpoint
        handle.claim_interface(0)?;


        Ok(
            Self {
                device_handle: handle,
                interface_number: interface.interface_number(),
                seek_position: 0
            }
        )
    }

    pub fn reset(&self) -> bool {
        let request_type = request_type(Direction::Out, rusb::RequestType::Class, rusb::Recipient::Interface);
        self.device_handle
            .write_control(request_type, 0xFF, 0, self.interface_number.into(), &[], Duration::from_secs(1))
            .is_ok()
    }

    pub fn max_lun(&self) -> Result<u8> {
        let request_type = request_type(Direction::In, rusb::RequestType::Class, rusb::Recipient::Interface);
        let mut buf: [u8; 1] = [0; 1];
        self.device_handle
            .read_control(request_type, 0xFE, 0, self.interface_number.into(), &mut buf, Duration::from_secs(1))?;

        Ok(buf[0])
    }
}

impl Read for MassStorageDevice {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        println!("{:?}", buf);
        todo!()
    }
}

impl Seek for MassStorageDevice {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        println!("{:?}", pos);
        self.seek_position = match pos {
            std::io::SeekFrom::Start(position) => position,
            std::io::SeekFrom::End(_) => todo!(),
            std::io::SeekFrom::Current(position) => self.seek_position.saturating_add_signed(position)
        };

        Ok(self.seek_position)
    }
}
