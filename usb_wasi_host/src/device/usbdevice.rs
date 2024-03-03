use crate::bindings::component::usb as world;
use anyhow::{Error, Result};
use async_trait::async_trait;
use rusb::{Device, UsbContext};
use std::time::Duration;
use wasmtime::component::{Resource, ResourceTableError};
use wasmtime_wasi::preview2::WasiView;

use world::device::{HostUsbDevice, HostDeviceHandle};
use world::types::{Configuration, Interface, InterfaceDescriptor, Properties, UsbError, DeviceHandleError};

#[derive(Debug)]
pub struct MyDeviceHandle {
    handle: rusb::DeviceHandle<rusb::Context>
}

impl From<rusb::Error> for DeviceHandleError {
    fn from(e: rusb::Error) -> Self {
        match e {
            rusb::Error::Io => DeviceHandleError::Io,
            rusb::Error::InvalidParam => DeviceHandleError::InvalidParam,
            rusb::Error::Access => DeviceHandleError::Access,
            rusb::Error::NoDevice => DeviceHandleError::NoDevice,
            rusb::Error::NotFound => DeviceHandleError::NotFound,
            rusb::Error::Busy => DeviceHandleError::Busy,
            rusb::Error::Timeout => DeviceHandleError::Timeout,
            rusb::Error::Overflow => DeviceHandleError::Overflow,
            rusb::Error::Pipe => DeviceHandleError::Pipe,
            rusb::Error::Interrupted => DeviceHandleError::Interrupted,
            rusb::Error::NoMem => DeviceHandleError::NoMem,
            rusb::Error::NotSupported => DeviceHandleError::NotSupported,
            rusb::Error::BadDescriptor => DeviceHandleError::BadDescriptor,
            rusb::Error::Other => DeviceHandleError::Other,
        }
    }
}

#[async_trait]
impl<T> HostDeviceHandle for T
where
    T: WasiView
{
    fn drop(&mut self, rep: Resource<MyDeviceHandle>) -> Result<()>  {
        Ok(self.table().delete(rep).map(|_| ())?)
    }

    async fn set_configuration(&mut self, handle: Resource<MyDeviceHandle>, configuration: u8) -> Result<()> {
        let _ = self.table()
            .get_mut(&handle)?
            .handle
            .set_active_configuration(configuration)
            .map_err(|e| println!("{:?}", e));

        Ok(())
    }

    async fn claim_interface(&mut self, handle: Resource<MyDeviceHandle>, interface: u8) -> Result<()> {
        let _ = self.table()
            .get_mut(&handle)?
            .handle
            .claim_interface(interface)
            .map_err(|e| println!("{:?}", e));

        Ok(())
    }

    async fn unclaim_interface(&mut self, handle: Resource<MyDeviceHandle>, interface: u8) -> Result<()> {
        let _ = self.table()
            .get_mut(&handle)?
            .handle
            .release_interface(interface)
            .map_err(|e| println!("{:?}", e));

        Ok(())
    }

    async fn write_interrupt(&mut self, handle: Resource<MyDeviceHandle>, endpoint: u8, data: Vec<u8>) -> Result<Result<u64, DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .write_interrupt(endpoint, &data, Duration::from_millis(10000))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result)
    }

    async fn write_bulk(&mut self, handle: Resource<MyDeviceHandle>, endpoint: u8, data: Vec<u8>) -> Result<Result<u64, DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .write_bulk(endpoint, &data, Duration::from_millis(10000))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result)
    }

    async fn read_bulk(&mut self, handle: Resource<MyDeviceHandle>, endpoint: u8) -> Result<Result<(u64, Vec<u8>), DeviceHandleError>> {
        let mut data = Vec::new();
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .read_bulk(endpoint, &mut data, Duration::from_millis(10))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result.map(|a| (a, data)))
    }
}

#[derive(Debug)]
pub struct MyDevice<T: rusb::UsbContext> {
    pub device: rusb::Device<T>,
}


impl<T> MyDevice<T> where T: rusb::UsbContext {
    fn get_language(handle: &rusb::DeviceHandle<T>, timeout: Duration) -> Result<rusb::Language> {
        let languages = handle.read_languages(timeout)?;
        let language = languages
            .first()
            .ok_or(Error::msg("No language to read configuration"))?;

        Ok(*language)
    }
}

impl<T> MyDevice<T> where T: rusb::UsbContext {
    fn get_properties(&self) -> Result<Properties> {
        let device = &self.device;
        let d = device.device_descriptor()?;
        let props = Properties {
            device_class: d.class_code(),
            device_protocol: d.protocol_code(),
            device_subclass: d.sub_class_code(),
            device_version: d.device_version().into(),
            product_id: d.product_id(),
            usb_version: d.usb_version().into(),
            vendor_id: d.vendor_id(),
        };
        Ok(props)
    }

    fn get_name(&self) -> Result<String> {
        let device = &self.device;
        let d = device.device_descriptor()?;
        let handle = device.open()?;

        let timeout = Duration::from_secs(1);

        let language = MyDevice::get_language(&handle, timeout)?;

        let device_name = handle.read_product_string(language, &d, timeout)?;

        Ok(device_name)
    }

    fn get_configurations(&self) -> Result<Vec<Configuration>> {
        let device = &self.device;

        let handle = device.open()?;

        let timeout = Duration::from_secs(1);

        let languages = handle.read_languages(timeout)?;
        let language = languages
            .first()
            .ok_or(Error::msg("No language to read configuration"))?;

        (0..device.device_descriptor()?.num_configurations())
            .map(|i| {
                let config = device.config_descriptor(i)?;
                let name = handle
                    .read_configuration_string(*language, &config, timeout)
                    .ok();

                let interfaces = config
                    .interfaces()
                    .map(|interface| {
                        let descriptors = interface
                            .descriptors()
                            .map(|d| InterfaceDescriptor {
                                class_code: d.class_code(),
                                endpoint_descriptors: d
                                    .endpoint_descriptors()
                                    .map(|ed| ed.into())
                                    .collect(),
                            })
                            .collect();

                        Interface {
                            number: interface.number(),
                            descriptors,
                        }
                    })
                    .collect();



                Ok(Configuration {
                    name,
                    max_power: config.max_power(),
                    number: config.number(),
                    interfaces,
                })
            })
            .collect()
    }
}

#[async_trait]
impl<T> HostUsbDevice for T
where
    T: WasiView
{
    fn drop(&mut self, rep: Resource<MyDevice<rusb::Context>>) -> Result<()> {
        Ok(self.table().delete(rep).map(|_| ())?)
    }

    async fn properties(&mut self, device: Resource<MyDevice<rusb::Context>>) -> Result<Properties> {
        self
            .table()
            .get(&device)?
            .get_properties()
    }

    async fn configurations(&mut self, device: Resource<MyDevice<rusb::Context>>) -> Result<Result<Vec<Configuration>, UsbError>> {
        let result = self
            .table()
            .get(&device)?
            .get_configurations()
            .map_err(|_| UsbError::DeviceDisconnected);

        Ok(result)
    }

    async fn get_name(&mut self, device: Resource<MyDevice<rusb::Context>>) -> Result<Result<String, UsbError>> {
        let result = self
            .table()
            .get(&device)?
            .get_name()
            .map_err(|_| UsbError::DeviceDisconnected);

        Ok(result)
    }

    async fn open(&mut self, device: Resource<MyDevice<rusb::Context>>) -> Result<Result<Resource<MyDeviceHandle>, DeviceHandleError>> {
        let handle = self
            .table()
            .get(&device)?
            .device
            .open()?;

        let resource = self
            .table()
            .push(MyDeviceHandle {handle})?;

        Ok(Ok(resource))
    }
}

#[async_trait]
impl<T> world::device::Host for T
where
    T: WasiView
{
    async fn get_devices(&mut self) -> Result<Vec<Resource<MyDevice<rusb::Context>>>> {
        let context = rusb::Context::new();
        let devices = context.unwrap().devices()?;

        devices
            .iter()
            .map(|device| {
                self
                    .table()
                    .push(MyDevice { device })
                    .map_err(Error::from)
            })
            .collect()
    }
}
