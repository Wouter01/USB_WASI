use tokio::sync::mpsc;
use rusb::{UsbContext, Hotplug};
use crate::device::usbdevice::MyDevice;

struct DeviceUpdateHandler {
	sender: mpsc::Sender<DeviceConnectionEvent>
}


impl DeviceUpdateHandler where {
	fn start_listener(self) {
		let context = rusb::Context::new().unwrap();
		let reg: Result<rusb::Registration<rusb::Context>, rusb::Error> = rusb::HotplugBuilder::new()
			.enumerate(true)
			.register(&context, Box::new(self));
	
		tokio::task::spawn_blocking(move || {
			let _reg = Some(reg.unwrap());
			loop {
				if let Err(_) = context.handle_events(None) {
					break;
				}
			}
		});
	}
}

// #[derive(Debug)]
pub enum DeviceConnectionEvent {
	Connected(MyDevice<rusb::Context>),
	Disconnected(MyDevice<rusb::Context>)
}

impl Hotplug<rusb::Context> for DeviceUpdateHandler {
	fn device_arrived(&mut self, device: rusb::Device<rusb::Context>) {
		let sender = self.sender.clone();
		
		let mydevice = MyDevice { device };		
		// sender.blocking_send cannot be used here, so a new task is created.
		// Blocking send will cause the main thread to panic.
		tokio::spawn(async move {
			let _ = sender.send(DeviceConnectionEvent::Connected(mydevice)).await;
		});
	}

	fn device_left(&mut self, device: rusb::Device<rusb::Context>) {
		let sender = self.sender.clone();
		
		let mydevice = MyDevice { device };
	
		tokio::spawn(async move {
			let _ = sender.send(DeviceConnectionEvent::Disconnected(mydevice)).await;
		});
	}
}

pub fn device_connection_updates() -> mpsc::Receiver<DeviceConnectionEvent> {
	let (sender, receiver) = mpsc::channel::<DeviceConnectionEvent>(10);
	
	let handler = DeviceUpdateHandler { sender };
	handler.start_listener();
	
	receiver
}