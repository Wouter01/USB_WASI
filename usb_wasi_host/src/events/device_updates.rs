use anyhow::Result;
use tokio::sync::mpsc;
use rusb::{UsbContext, Hotplug};
use crate::device::usbdevice::MyDevice;

struct DeviceUpdateHandler {
	sender: mpsc::Sender<DeviceConnectionEvent>
}

impl DeviceUpdateHandler {
	fn new(buffer_size: usize) -> Result<(mpsc::Receiver<DeviceConnectionEvent>, rusb::Registration<rusb::Context>, tokio::task::JoinHandle<()>)> {
		let (sender, receiver) = mpsc::channel::<DeviceConnectionEvent>(buffer_size);
		
		let handler = DeviceUpdateHandler {
			sender
		};
		
		let (registration, task) = handler.start_listener()?;
		Ok((receiver, registration, task))
	}
}

impl DeviceUpdateHandler where {
	fn start_listener(self) -> Result<(rusb::Registration<rusb::Context>, tokio::task::JoinHandle<()>)> {
		let context = rusb::Context::new().unwrap();
		let reg: Result<rusb::Registration<rusb::Context>, rusb::Error> = rusb::HotplugBuilder::new()
			.enumerate(true)
			.register(&context, Box::new(self));
	
		let task = tokio::task::spawn_blocking(move || {
			loop {
				if let Err(e) = context.handle_events(None) {
					println!("Got Error! {:?}", e);
					break;
				}
			}
		});
		
		match reg {
			Err(e) => Err(anyhow::Error::new(e)),
			Ok(a) => Ok((a, task))
		}
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

pub fn device_connection_updates() -> Result<(mpsc::Receiver<DeviceConnectionEvent>, rusb::Registration<rusb::Context>, tokio::task::JoinHandle<()>)> {
	// let (sender, receiver) = mpsc::channel::<DeviceConnectionEvent>(10);
	
	let receiver = DeviceUpdateHandler::new(10)?;
	
	// let handler = DeviceUpdateHandler { sender };
	// handler.start_listener();
	
	Ok(receiver)
}