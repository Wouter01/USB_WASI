pub struct DeviceFilter {
	vendor_id: Option<u16>,
	product_id: Option<u16>,
	serial_number: Option<String>,
	classCode: String,
	subclassCode: String,
	protocolCode: String
}

impl DeviceFilter {
	// fn matches_device(&self, device: &rusb::Device<rusb::GlobalContext>) -> bool {
	// 	let descriptor = {
	// 		let descriptor = device.device_descriptor();
	// 		match descriptor {
	// 			Ok(x) => x,
	// 			Err(_) => return false
	// 		}
	// 	};
	// 	
	// 	if let Some(vendor_id) = self.vendor_id {
	// 		if vendor_id != descriptor.vendor_id() {
	// 			return false;
	// 		}
	// 	}
	// 	
	// 	if let Some(product_id) = self.product_id {
	// 		if product_id != descriptor.product_id() {
	// 			return false;
	// 		}
	// 	}
	// 	
	// 	if let Some(serial_number) = self.serial_number.as_deref() {
	// 		let handle = device.open().unwrap();
	// 		let sn_res = handle.read_serial_number_string_ascii(&descriptor);
	// 		let sn = match sn_res {
	// 			Ok(x) => x,
	// 			Err(_) => return false
	// 		};
	// 		if serial_number != sn {
	// 			return false;
	// 		}
	// 	}
	// 	
	// 	return true;
	// }
}

// pub fn get_device_list(filters: Vec<DeviceFilter>) -> Vec<u8> {
// 	println!("Getting Device List...");
// 	let devices = rusb::devices();
// 	if let Ok(devices) = devices {
// 		devices
// 			.iter()
// 			.filter_map(|device| {
// 				println!("Device: {:?}", device);
// 				if filters.iter().find(|d| d.matches_device(&device)).is_some() {
// 					return Some(device.address());
// 				}
// 
// 				return None;
// 			})
// 			.collect()
// 	} else {
// 		vec![]
// 	}
// }

pub struct UsbDevice {
	
}