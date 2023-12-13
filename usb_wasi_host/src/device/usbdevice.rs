use anyhow::{Result, Error};
use std::time::Duration;
use async_trait::async_trait;
use wasmtime_wasi::preview2::WasiView;

use crate::bindings::component::usb::types::{Properties, Configuration, Interface, InterfaceDescriptor};

use crate::bindings::component::usb::device::{HostUsbDevice, UsbDevice};
use wasmtime::component::Resource;

#[derive(Debug)]
pub struct MyDevice {
	pub device: rusb::Device<rusb::GlobalContext>
}

impl MyDevice {
	
	fn get_properties(&self) -> Result<Properties> {
		let device = &self.device;
		Ok(device.device_descriptor()?.into())
	}
	
	fn get_configurations(&self) -> Result<Vec<Configuration>> {
		
		let device = &self.device;
		
		let handle = device.open()?;
		
		let timeout = Duration::from_secs(1);
		
		let languages = handle.read_languages(timeout)?;
		let language = languages.first().ok_or(Error::msg("No language to read configuration"))?;
		
		(0..device.device_descriptor()?.num_configurations())
		.map(|i| {
			let config = device.config_descriptor(i)?;
			let name = handle.read_configuration_string(*language, &config, timeout).ok();
			
			
			
			let interfaces = config.interfaces().map(|interface| {
				let descriptors = interface
				.descriptors()
				.map(|d| {
					let endpoint_descriptors = d
					.endpoint_descriptors()
					.map(|ed| ed.into())
					.collect();
					
				   InterfaceDescriptor {
					   class_code: d.class_code(),
					   endpoint_descriptors
					   
				   }
				})
				.collect();
				
				Interface {
					number: interface.number(),
					descriptors
				}
			})
			.collect();
			
			Ok(Configuration { 
				name,
				max_power: config.max_power(),
				interfaces
			})
		})
		.collect()
	}
}

#[async_trait]
impl<T> HostUsbDevice for T where T: WasiView {
	
	fn drop(&mut self, rep: Resource<UsbDevice>) -> Result<()> {
		Ok(self
		.table_mut()
		.delete(rep)
		.map(|_| ())?)
	}
	
	async fn properties(&mut self, device: Resource<UsbDevice>) -> Result<Properties> {
		self.table().get(&device)?.get_properties()
	}
	
	async fn configurations(&mut self, device: Resource<UsbDevice>) -> Result<Vec<Configuration>> {
		self.table().get(&device)?.get_configurations()
	}
}