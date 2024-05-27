use std::{io::{self, Read, Seek, Write}, time::{Duration, Instant}};
use anyhow::{anyhow, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::command_wrapper::{CBWDirection, CSWStatus, CommandBlockWrapper, CommandStatusWrapper, DeviceCapacity};

#[cfg(not(target_arch = "wasm32"))]
use rusb::{request_type, Context, DeviceHandle, Direction, TransferType, UsbContext};

#[cfg(target_arch = "wasm32")]
use crate::bindings::component::usb::{descriptors::*, usb::*};


// use

pub struct MassStorageDevice {
    pub capacity: DeviceCapacity,

    #[cfg(not(target_arch = "wasm32"))]
    device_handle: DeviceHandle<Context>,
    #[cfg(target_arch = "wasm32")]
    device_handle: DeviceHandle,

    interface_number: u8,
    endpoint_in: u8,
    endpoint_out: u8,
    seek_position: u64,
    tag: u32
}

#[cfg(not(target_arch = "wasm32"))]
const DURATION: Duration = Duration::from_secs(1);

#[cfg(target_arch = "wasm32")]
const DURATION: u64 = 1_000_000_000; // 1 second

impl MassStorageDevice {
    fn increase_tag(&mut self) -> u32 {
        self.tag += 1;
        self.tag
    }

    #[cfg(not(target_arch = "wasm32"))]
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
            .find_map(|i| {
                i.descriptors().find(|i| i.class_code() == 0x08 && i.protocol_code() == 0x50)
            })
            .ok_or(anyhow!("No mass storage interface found."))?;

        // println!("{}", interface.interface_number());
        let endpoint_in = interface
            .endpoint_descriptors()
            .find(|e| e.transfer_type() == TransferType::Bulk && e.direction() == Direction::In)
            .ok_or(anyhow!("No Bulk In Endpoint found,"))?;

        let endpoint_out = interface
            .endpoint_descriptors()
            .find(|e| e.transfer_type() == TransferType::Bulk && e.direction() == Direction::Out)
            .ok_or(anyhow!("No Bulk Out Endpoint found,"))?;

        let mut handle = device.open()?;
        handle.set_auto_detach_kernel_driver(true)?;
        handle.reset()?;
        handle.set_active_configuration(0)?;

        // Claim interface with bulk endpoints
        handle.claim_interface(interface.interface_number())?;

        let mut _self = Self {
            device_handle: handle,
            interface_number: interface.interface_number(),
            seek_position: 0,
            endpoint_in: endpoint_in.address(),
            endpoint_out: endpoint_out.address(),
            tag: 0,
            capacity: Default::default()
        };

        if !_self.reset() {
            return Err(anyhow!("Could not reset device"));
        };

        assert!(_self.test_unit_ready()?);

        _self.capacity = _self.read_capacity()?;

        Ok(_self)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Result<Self> {
        let devices = UsbDevice::enumerate();

        let (device, interface) = devices
            .iter()
            .find_map(|device| {
                let interface = device.configurations()
                    .unwrap()[0]
                    .interfaces
                    .iter()
                    .find(|i| i.class_code == 0x08 && i.protocol == 0x50)
                    .map(|i| i.to_owned());

                interface.map(|i| (device, i))
            })
            .expect("No mass storage interface found.");

        // println!("{}", interface.number);
        let endpoint_in = interface
            .endpoint_descriptors
            .iter()
            .find(|e| e.transfer_type == TransferType::Bulk && e.direction == Direction::In)
            .ok_or(anyhow!("No Bulk In Endpoint found,"))?;

        let endpoint_out = interface
            .endpoint_descriptors
            .iter()
            .find(|e| e.transfer_type == TransferType::Bulk && e.direction == Direction::Out)
            .ok_or(anyhow!("No Bulk Out Endpoint found,"))?;

        let mut handle = device.open()?;
        handle.reset()?;
        handle.select_configuration(0)?;

        // Claim interface with bulk endpoints
        handle.claim_interface(interface.number)?;

        let mut _self = Self {
            device_handle: handle,
            interface_number: interface.number,
            seek_position: 0,
            endpoint_in: endpoint_in.address,
            endpoint_out: endpoint_out.address,
            tag: 0,
            capacity: Default::default()
        };

        if !_self.reset() {
            return Err(anyhow!("Could not reset device"));
        };

        assert!(_self.test_unit_ready()?);

        _self.capacity = _self.read_capacity()?;

        Ok(_self)
    }

    fn test_unit_ready(&mut self) -> Result<bool> {
        self.send_over_usb(vec![0; 6], None)
            .map(|status| status.status == CSWStatus::Passed)
    }

    fn read_capacity(&mut self) -> Result<DeviceCapacity> {
        let mut data: [u8; 8] = [0; 8];
        let mut command_block = BytesMut::with_capacity(11);
        command_block.put_u8(0x25);
        command_block.put_bytes(0, 10);

        self.receive_over_usb(command_block.to_vec(), &mut data)
            .map(|_| DeviceCapacity::from(Bytes::copy_from_slice(&data)))
    }

    fn reset(&self) -> bool {
        // let request_type = request_type(Direction::Out, rusb::RequestType::Class, rusb::Recipient::Interface);
        // dbg!(request_type);
        let request_type = 33;
        self.device_handle
            .write_control(request_type, 0xFF, 0, self.interface_number.into(), &[], DURATION)
            .is_ok()
    }

    pub fn read_10(&mut self, block_address: u32, transfer_length: u16) -> Result<Vec<u8>> {
        let mut command_block = BytesMut::with_capacity(11);
        command_block.put_u8(0x28);
        command_block.put_u8(0);
        command_block.put_u32(block_address);
        command_block.put_u8(0);
        command_block.put_u16(transfer_length);
        command_block.put_u8(0);
        command_block.put_u16(0);

        let capacity = transfer_length as u64 * self.capacity.block_length as u64;
        let mut data: Vec<u8> = vec![0; capacity as usize];
        let _ = self.receive_over_usb(command_block.to_vec(), &mut data)?;

        Ok(data)
    }

    fn send_cbw_request(&mut self, direction: CBWDirection, data_transfer_length: u32, cbwcb: Vec<u8>) -> Result<u32> {
        let tag = self.increase_tag();

        let cbw: Vec<u8> = CommandBlockWrapper {
            tag,
            data_transfer_length,
            direction,
            lun: 0,
            cbwcb
        }.into();

        self.device_handle.write_bulk(self.endpoint_out, &cbw, DURATION)?;

        Ok(tag)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn receive_csw(&self, tag: u32) -> Result<CommandStatusWrapper> {
        let mut status_data: [u8; 13] = [0; 13];
        self.device_handle.read_bulk(self.endpoint_in, &mut status_data, DURATION)?;

        let status_bytes: Bytes = status_data.to_vec().into();
        let status = CommandStatusWrapper::from(status_bytes);

        assert!(status.tag == tag && status.status == CSWStatus::Passed);

        Ok(status)
    }

    #[cfg(target_arch = "wasm32")]
    fn receive_csw(&self, tag: u32) -> Result<CommandStatusWrapper> {
        let mut status_data: [u8; 13] = [0; 13];
        let (bytes_written, data) = self.device_handle.read_bulk(self.endpoint_in, 13, DURATION)?;
        status_data.copy_from_slice(&data);

        let status_bytes: Bytes = status_data.to_vec().into();
        let status = CommandStatusWrapper::from(status_bytes);

        assert!(status.tag == tag && status.status == CSWStatus::Passed);

        Ok(status)
    }


    #[cfg(not(target_arch = "wasm32"))]
    fn receive_over_usb(&mut self, cbwcb: Vec<u8>, data: &mut [u8]) -> Result<CommandStatusWrapper> {
        // let now = Instant::now();

        let tag = self.send_cbw_request(CBWDirection::In, data.len() as u32, cbwcb)?;

        self.device_handle.read_bulk(self.endpoint_in, data, DURATION)?;

        self.receive_csw(tag)
        // dbg!(now.elapsed());
        // result
    }

    #[cfg(target_arch = "wasm32")]
    fn receive_over_usb(&mut self, cbwcb: Vec<u8>, data: &mut [u8]) -> Result<CommandStatusWrapper> {
        // let now = Instant::now();
        let max_size = data.len() as u32;
        let tag = self.send_cbw_request(CBWDirection::In, max_size, cbwcb)?;

        let (bytes_received, bytes) = self.device_handle.read_bulk(self.endpoint_in, max_size as u64, DURATION)?;

        data[..bytes_received as usize].copy_from_slice(&bytes);

        self.receive_csw(tag)
        // dbg!(now.elapsed());
        // result
    }

    fn send_over_usb(&mut self, cbwcb: Vec<u8>, data: Option<&[u8]>) -> Result<CommandStatusWrapper> {
        let length = data.map(|d| d.len()).unwrap_or(0) as u32;
        let tag = self.send_cbw_request(CBWDirection::Out, length, cbwcb)?;

        if let Some(data) = data {
            self.device_handle.write_bulk(self.endpoint_out, data, DURATION)?;
        }

        self.receive_csw(tag)
    }
}

impl Read for MassStorageDevice {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Do nothing when buffer is empty.
        if buf.is_empty() {
            return Ok(0);
        }

        let start_address = self.seek_position;
        let end_address = self.seek_position + buf.len() as u64;

        assert!(end_address <= self.capacity.size);

        let num_bytes = (end_address - start_address) as usize;
        let block_length = self.capacity.block_length as u64;

        // Address of the block which contains the start address.
        let block_address = start_address / block_length;
        // The amount of blocks that will be read.
        let transfer_length = (end_address - 1) / block_length - block_address + 1;

        let data = self.read_10(block_address as u32, transfer_length as u16)
            .map_err(|e| io::Error::other(e))?;

        // Data contains whole blocks, we may need to cut off some data at start and end.
        let start_index = (start_address % block_length) as usize;
        let end_index = start_index + num_bytes;
        buf.copy_from_slice(&data[start_index..end_index]);

        self.seek_position += num_bytes as u64;
        Ok(num_bytes)
    }
}

impl Seek for MassStorageDevice {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.seek_position = match pos {
            std::io::SeekFrom::Start(position) => position,
            std::io::SeekFrom::End(position) => self.capacity.size + position as u64,
            std::io::SeekFrom::Current(position) => self.seek_position.saturating_add_signed(position)
        };

        Ok(self.seek_position)
    }
}
