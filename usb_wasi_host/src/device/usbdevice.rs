use crate::bindings::component::usb as world;
use crate::usb_host_wasi_view::USBHostWasiView;
use anyhow::{Error, Result};
use async_trait::async_trait;
use rusb::{ConfigDescriptor, DeviceHandle as RusbDeviceHandle, Language, UsbContext};
use std::time::Duration;
use wasmtime::component::Resource;
use wasmtime_wasi::WasiView;

use world::usb::HostUsbDevice;
use world::descriptors::{ConfigurationDescriptor, InterfaceDescriptor, DeviceDescriptor};
use world::types::DeviceHandleError;

use super::devicehandle::DeviceHandle;

#[derive(Debug)]
pub struct USBDevice {
    pub device: rusb::Device<rusb::Context>,
}

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

impl USBDevice {
    fn get_language(handle: &rusb::DeviceHandle<rusb::Context>, timeout: Duration) -> Result<rusb::Language> {
        let languages = handle.read_languages(timeout)?;
        let language = languages
            .first()
            .ok_or(Error::msg("No language to read configuration"))?;

        Ok(*language)
    }
}

impl USBDevice {
    fn read_property<S>(&self, perform: impl FnOnce(&rusb::Device<rusb::Context>, RusbDeviceHandle<rusb::Context>, &Language) -> Result<S, rusb::Error>) -> Result<S, DeviceHandleError> {
        let device = &self.device;
        let handle = device.open().map_err(DeviceHandleError::from)?;
        let languages = handle.read_languages(DEFAULT_TIMEOUT).map_err(DeviceHandleError::from)?;
        let language = languages
            .first()
            .ok_or(DeviceHandleError::Other)?;

        perform(device, handle, language)
            .map_err(DeviceHandleError::from)
    }

    fn get_configuration(&self, config: ConfigDescriptor) -> ConfigurationDescriptor {

        let interfaces = config
            .interfaces()
            .flat_map(|interface| interface.descriptors())
            .map(|descriptor| {
                InterfaceDescriptor {
                    number: descriptor.interface_number(),
                    alternate_setting: descriptor.setting_number(),
                    class_code: descriptor.class_code(),
                    subclass_code: descriptor.sub_class_code(),
                    protocol: descriptor.protocol_code(),
                    interface_string_index: descriptor.description_string_index(),
                    endpoint_descriptors: descriptor.endpoint_descriptors().map(|e| e.into()).collect(),
                }
            })
            .collect();

        let configuration = ConfigurationDescriptor {
            max_power: config.max_power(),
            number: config.number(),
            interfaces,
        };

        configuration
    }
}

#[async_trait]
impl HostUsbDevice for USBHostWasiView {
    fn drop(&mut self, rep: Resource<USBDevice>) -> Result<()> {
        Ok(self.table().delete(rep).map(|_| ())?)
    }

    async fn device_descriptor(&mut self, device: Resource<USBDevice>) -> Result<DeviceDescriptor> {
        let descriptor = self
            .table()
            .get(&device)?
            .device.device_descriptor()?
            .into();

        Ok(descriptor)
    }

    async fn configurations(&mut self, device: Resource<USBDevice>) -> Result<Result<Vec<ConfigurationDescriptor>, DeviceHandleError>> {
        let resource = self
            .table()
            .get(&device)?;

        let device = &resource.device;

        let config_count = device.device_descriptor()?.num_configurations();
        let mut configurations: Vec<ConfigurationDescriptor> = Vec::with_capacity(config_count.into());

        for i in 0..config_count {
            match device.config_descriptor(i) {
                Ok(config) => {
                    let resource = resource.get_configuration(config);
                    configurations.push(resource)
                }
                Err(error) => return Ok(Err(error.into()))
            }
        }

        Ok(Ok(configurations))
    }

    // async fn configurations(&mut self, device: Resource<MyDevice<rusb::Context>>) -> Result<Result<ConfigurationDescriptor, DeviceHandleError>> {
    //     let resource = self
    //         .table()
    //         .get(&device)?;

    //     let config = resource.read_property(|device, handle, language| {
    //         let config = device.active_config_descriptor()?;
    //         Ok(resource.get_configuration(&handle, config, language))
    //     });

    //     Ok(config)
    // }

    // async fn product_name(&mut self, device: Resource<MyDevice<rusb::Context>>) -> Result<Result<String, DeviceHandleError>> {
    //     let device = self
    //         .table()
    //         .get(&device)?;

    //     let name = device.read_property(|device, handle, language| {
    //         let descriptor = device.device_descriptor()?;
    //         handle.read_product_string(*language, &descriptor, DEFAULT_TIMEOUT)
    //     });

    //     Ok(name)
    // }

    // async fn manufacturer_name(&mut self, device: Resource<MyDevice<rusb::Context>>) -> Result<Result<String, DeviceHandleError>> {
    //     let device = self
    //         .table()
    //         .get(&device)?;

    //     let name = device.read_property(|device, handle, language| {
    //         let descriptor = device.device_descriptor()?;
    //         handle.read_manufacturer_string(*language, &descriptor, DEFAULT_TIMEOUT)
    //     });

    //     Ok(name)
    // }

    // async fn serial_number(&mut self, device: Resource<MyDevice<rusb::Context>>) -> Result<Result<String, DeviceHandleError>> {
    //     let device = self
    //         .table()
    //         .get(&device)?;

    //     let name = device.read_property(|device, handle, language| {
    //         let descriptor = device.device_descriptor()?;
    //         handle.read_serial_number_string(*language, &descriptor, DEFAULT_TIMEOUT)
    //     });

    //     Ok(name)
    // }

    async fn open(&mut self, device: Resource<USBDevice>) -> Result<Result<Resource<DeviceHandle>, DeviceHandleError>> {
        let device_address = self
            .table()
            .get(&device)?
            .device
            .address();

        let already_present = !self.active_device_handles.insert(device_address);
        if already_present {
            return Ok(Err(DeviceHandleError::Access));
        }

        let mut handle = self
            .table()
            .get(&device)?
            .device
            .open()?;

        _ = handle.set_auto_detach_kernel_driver(true);

        let resource = self
            .table()
            .push(DeviceHandle {device_address, handle})?;

        Ok(Ok(resource))
    }

    async fn enumerate(&mut self) -> Result<Vec<Resource<USBDevice>>> {
        let context = rusb::Context::new();

        context?
            .devices()?
            .iter()
            .map(|device| {
                self
                    .table()
                    .push(USBDevice { device })
                    .map_err(Error::from)
            })
            .collect()
    }
}

// #[async_trait]
// impl<T> world::device::Host for T
// where
//     T: WasiView
// {
    // async fn get_devices(&mut self) -> Result<Vec<Resource<MyDevice<rusb::Context>>>> {
    //     let context = rusb::Context::new();

    //     context?
    //         .devices()?
    //         .iter()
    //         .map(|device| {
    //             self
    //                 .table()
    //                 .push(MyDevice { device })
    //                 .map_err(Error::from)
    //         })
    //         .collect()
    // }

    // async fn request_device(&mut self, filter: DeviceFilter) -> Result<Option<Resource<MyDevice<rusb::Context>>>> {
    //     let context = rusb::Context::new();

    //     let device = context?
    //         .devices()?
    //         .iter()
    //         .find(|device| {
    //             let Ok(descriptor) = device.device_descriptor() else { return false };

    //             filter.class_code.map_or(true, |v| descriptor.class_code() == v)
    //             && filter.subclass_code.map_or(true, |v| descriptor.sub_class_code() == v)
    //             && filter.product_id.map_or(true, |v| descriptor.product_id() == v)
    //             && filter.protocol_code.map_or(true, |v| descriptor.protocol_code() == v)
    //             && filter.vendor_id.map_or(true, |v| descriptor.vendor_id() == v)
    //             && filter.serial_number.as_ref().map_or(true, |v| {
    //                 let Ok(device) = device.open() else { return false };

    //                 let Ok(languages) = device.read_languages(DEFAULT_TIMEOUT) else { return false };
    //                 let Some(language) = languages.first() else { return false };

    //                 let Ok(serial_number) = device.read_serial_number_string(*language, &descriptor, DEFAULT_TIMEOUT) else { return false };

    //                 serial_number == *v
    //             })
    //         });

    //     let Some(device) = device else { return Ok(None) };

    //     let resource = self
    //         .table()
    //         .push(MyDevice { device })
    //         .map_err(Error::from)?;

    //     Ok(Some(resource))
    // }
// }
