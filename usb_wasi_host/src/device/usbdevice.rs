use crate::bindings::component::usb as world;
use anyhow::{Error, Result};
use async_trait::async_trait;
use rusb::UsbContext;
use std::time::Duration;
use wasmtime::component::Resource;
use wasmtime_wasi::preview2::WasiView;

use world::device::HostUsbDevice;
use world::types::{Configuration, Interface, InterfaceDescriptor, Properties, UsbError, DeviceHandleError, DeviceFilter};

use super::devicehandle::MyDeviceHandle;

#[derive(Debug)]
pub struct MyDevice<T: rusb::UsbContext> {
    pub device: rusb::Device<T>,
}

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

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

    fn get_active_configuration(&self) -> Result<Configuration> {
        let device = &self.device;

        let handle = device.open()?;

        let languages = handle.read_languages(DEFAULT_TIMEOUT)?;
        let language = languages
            .first()
            .ok_or(Error::msg("No language to read configuration"))?;

        let config = device.active_config_descriptor()?;

        let name = handle
            .read_configuration_string(*language, &config, DEFAULT_TIMEOUT)
            .ok();

        let interfaces = config
            .interfaces()
            .map(|interface| {
                Interface {
                    number: interface.number(),
                    descriptors: interface
                        .descriptors()
                        .map(|d| InterfaceDescriptor {
                            class_code: d.class_code(),
                            endpoint_descriptors: d
                                .endpoint_descriptors()
                                .map(|ed| ed.into())
                                .collect(),
                        })
                        .collect()
                }
            })
            .collect();

        let configuration = Configuration {
            name,
            max_power: config.max_power(),
            number: config.number(),
            interfaces,
        };

        Ok(configuration)

    }

    fn get_configurations(&self) -> Result<Vec<Configuration>> {
        let device = &self.device;

        let handle = device.open()?;



        let languages = handle.read_languages(DEFAULT_TIMEOUT)?;
        let language = languages
            .first()
            .ok_or(Error::msg("No language to read configuration"))?;

        let descriptor = device.device_descriptor()?;

        let mut configurations: Vec<Configuration> = Vec::with_capacity(descriptor.num_configurations().into());

        for i in 0..descriptor.num_configurations() {
            let config = device.config_descriptor(i)?;
            let name = handle
                .read_configuration_string(*language, &config, DEFAULT_TIMEOUT)
                .ok();

            let interfaces = config
                .interfaces()
                .map(|interface| {
                    Interface {
                        number: interface.number(),
                        descriptors: interface
                            .descriptors()
                            .map(|d| InterfaceDescriptor {
                                class_code: d.class_code(),
                                endpoint_descriptors: d
                                    .endpoint_descriptors()
                                    .map(|ed| ed.into())
                                    .collect(),
                            })
                            .collect()
                    }
                })
                .collect();

            let configuration = Configuration {
                name,
                max_power: config.max_power(),
                number: config.number(),
                interfaces,
            };

            configurations.push(configuration)
        }

        Ok(configurations)
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

    async fn configuration(&mut self, device: Resource<MyDevice<rusb::Context>>) -> Result<Result<Configuration, UsbError>> {
        let config = self
            .table()
            .get(&device)?
            .get_active_configuration()
            .map_err(|_| UsbError::ConfigReadError);

        Ok(config)
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
        let mut handle = self
            .table()
            .get(&device)?
            .device
            .open()?;

        handle.reset()?;

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

        context?
            .devices()?
            .iter()
            .map(|device| {
                self
                    .table()
                    .push(MyDevice { device })
                    .map_err(Error::from)
            })
            .collect()
    }

    async fn request_device(&mut self, filter: DeviceFilter) -> Result<Option<Resource<MyDevice<rusb::Context>>>> {
        let context = rusb::Context::new();

        let device = context?
            .devices()?
            .iter()
            .find(|device| {
                let Ok(descriptor) = device.device_descriptor() else { return false };

                filter.class_code.map_or(true, |v| descriptor.class_code() == v)
                && filter.subclass_code.map_or(true, |v| descriptor.sub_class_code() == v)
                && filter.product_id.map_or(true, |v| descriptor.product_id() == v)
                && filter.protocol_code.map_or(true, |v| descriptor.protocol_code() == v)
                && filter.vendor_id.map_or(true, |v| descriptor.vendor_id() == v)
                && filter.serial_number.as_ref().map_or(true, |v| {
                    let Ok(device) = device.open() else { return false };

                    let Ok(languages) = device.read_languages(DEFAULT_TIMEOUT) else { return false };
                    let Some(language) = languages.first() else { return false };

                    let Ok(serial_number) = device.read_serial_number_string(*language, &descriptor, DEFAULT_TIMEOUT) else { return false };

                    serial_number == *v
                })
            });

        let Some(device) = device else { return Ok(None) };

        let resource = self
            .table()
            .push(MyDevice { device })
            .map_err(Error::from)?;

        Ok(Some(resource))
    }
}
