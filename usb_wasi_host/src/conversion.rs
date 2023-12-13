use crate::bindings::component::usb::types::{Version, EndpointDescriptor, Direction, SyncType, UsageType, TransferType, Properties};

impl From<rusb::Version> for Version {
	fn from(a: rusb::Version) -> Self {
		Version { major: a.0, minor: a.1, subminor: a.2 }
	}
}

impl From<rusb::Direction> for Direction {
	fn from(b: rusb::Direction) -> Self {
		match b {
			rusb::Direction::In => Self::In,
			rusb::Direction::Out => Self::Out
		}
	}
}

impl From<rusb::SyncType> for SyncType {
	fn from(b: rusb::SyncType) -> Self {
		match b {
			rusb::SyncType::Adaptive => Self::Adaptive,
			rusb::SyncType::Asynchronous => Self::Asynchronous,
			rusb::SyncType::NoSync => Self::NoSync,
			rusb::SyncType::Synchronous => Self::Synchronous
		}
	}
}

impl From<rusb::UsageType> for UsageType {
	fn from(b: rusb::UsageType) -> Self {
		match b {
			rusb::UsageType::Data => Self::Data,
			rusb::UsageType::Feedback => Self::Feedback,
			rusb::UsageType::FeedbackData => Self::FeedbackData,
			rusb::UsageType::Reserved => Self::Reserved
		}
	}
}

impl From<rusb::TransferType> for TransferType {
	fn from(b: rusb::TransferType) -> Self {
		match b {
			rusb::TransferType::Bulk => Self::Bulk,
			rusb::TransferType::Control => Self::Control,
			rusb::TransferType::Interrupt => Self::Interrupt,
			rusb::TransferType::Isochronous => Self::Isochronous
		}
	}
}

impl From<rusb::EndpointDescriptor<'_>> for EndpointDescriptor {
	fn from(ed: rusb::EndpointDescriptor) -> Self {
		Self { 
			address: ed.address(),
			direction: ed.direction().into(),
			interval: ed.interval(),
			max_packet_size: ed.max_packet_size(),
			number: ed.number(),
			refresh: ed.refresh(),
			sync_type: ed.sync_type().into(),
			synch_address: ed.synch_address(),
			transfer_type: ed.transfer_type().into(),
			usage_type: ed.usage_type().into()
		}
	}
}

impl From<rusb::DeviceDescriptor> for Properties {
	fn from(d: rusb::DeviceDescriptor) -> Self {
		Self {
			device_class: d.class_code(),
			device_protocol: d.protocol_code(),
			device_subclass: d.sub_class_code(),
			device_version: d.device_version().into(),
			product_id: d.product_id(),
			usb_version: d.usb_version().into(),
			vendor_id: d.vendor_id()
		}
	}
}