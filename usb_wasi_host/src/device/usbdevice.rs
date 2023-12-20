use crate::bindings::component::usb as world;
use anyhow::{Error, Result};
use async_trait::async_trait;
use std::time::Duration;
use wasmtime::component::Resource;
use wasmtime_wasi::preview2::WasiView;

use world::device::HostUsbDevice;
use world::types::{Configuration, Interface, InterfaceDescriptor, Properties};

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
        let handle = device.open()?;
        
        let timeout = Duration::from_secs(1);
        
        let language = MyDevice::get_language(&handle, timeout)?;
            
        let device_name = handle.read_product_string(language, &d, timeout)?;
        
        let props = Properties {
            device_name: device_name,
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
    fn drop(&mut self, rep: Resource<MyDevice<rusb::GlobalContext>>) -> Result<()> {
        Ok(self.table_mut().delete(rep).map(|_| ())?)
    }

    async fn properties(&mut self, device: Resource<MyDevice<rusb::GlobalContext>>) -> Result<Properties> {
        self.table().get(&device)?.get_properties()
    }

    async fn configurations(&mut self, device: Resource<MyDevice<rusb::GlobalContext>>) -> Result<Vec<Configuration>> {
        self.table().get(&device)?.get_configurations()
    }
}

#[async_trait]
impl<T> world::device::Host for T
where
    T: WasiView
{
    async fn get_devices(&mut self) -> Result<Vec<Resource<MyDevice<rusb::GlobalContext>>>> {
        rusb::devices()?
            .iter()
            .map(|device| {
                self.table_mut()
                    .push(MyDevice { device })
                    .map_err(Error::from)
            })
            .collect()
    }
}
