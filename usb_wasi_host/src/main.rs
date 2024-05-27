use anyhow::Result;
use clap::Parser;
use usb_host_wasi_view::USBHostWasiView;
use wasmtime_wasi::bindings::Command;
use std::{path::PathBuf, process::exit, str::FromStr};
use wasmtime::{component::*, Config, Engine, Store};

use crate::bindings::Imports;

mod conversion;
mod device;
mod events;
mod usb_host_wasi_view;

pub mod bindings {
    wasmtime::component::bindgen!({
        world: "component:usb/imports",
        async: true,
        with: {
            "component:usb/usb/usb-device": crate::device::usbdevice::USBDevice,
            "component:usb/usb/device-handle": crate::device::devicehandle::DeviceHandle,
        },
        path: "../WIT/wit"
    });
}

#[derive(Parser)]
#[clap(name = "usb", version = env!("CARGO_PKG_VERSION"))]
struct UsbDemoAppParser {
    /// The path to the guest component.
    #[clap(value_name = "COMPONENT_PATH")]
    component_path: PathBuf,

    /// Comma-separated list of USB devices to allow (in hex format: vendor_id:product_id, e.g. 12AB:34CD).
    #[clap(long, value_name = "USB_DEVICES", use_value_delimiter = true)]
    usb_devices: Vec<USBDeviceIdentifier>,

    /// Use a denylist for USB devices instead of an allowlist.
    #[clap(long)]
    usb_use_denylist: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct USBDeviceIdentifier {
    vendor_id: u16,
    product_id: u16
}

impl FromStr for USBDeviceIdentifier {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid format. Expected vendor_id:product_id");
        }

        let vendor_id = u16::from_str_radix(parts[0], 16).map_err(|_| "Invalid vendor_id")?;
        let product_id = u16::from_str_radix(parts[1], 16).map_err(|_| "Invalid product_id")?;

        Ok(Self { vendor_id, product_id })
    }
}

#[derive(Debug, Clone)]
enum AllowedUSBDevices {
    Allowed(Vec<USBDeviceIdentifier>),
    Denied(Vec<USBDeviceIdentifier>)
}

impl AllowedUSBDevices {
    fn is_allowed(&self, device: &USBDeviceIdentifier) -> bool {
        match self {
            Self::Allowed(devices) => devices.contains(device),
            Self::Denied(devices) => !devices.contains(device)
        }
    }
}

struct UsbDemoApp {
    engine: Engine,
    linker: Linker<USBHostWasiView>,
    component: Component
}

impl UsbDemoApp {
    fn new(component: PathBuf) -> Result<Self> {
        let mut config = Config::default();
        config.wasm_component_model(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);

        wasmtime_wasi::add_to_linker_async(&mut linker)?;
        Imports::add_to_linker(&mut linker, |view| view)?;

        let component = Component::from_file(&engine, component)?;

        Ok(Self {
            engine,
            linker,
            component
        })
    }

    async fn start(&mut self, allowed_devices: AllowedUSBDevices) -> anyhow::Result<Result<(), ()>> {
        let data = USBHostWasiView::new(allowed_devices)?;
        let mut store = Store::new(&self.engine, data);

        let (command, _) = Command::instantiate_async(&mut store, &self.component, &self.linker).await?;

        command.wasi_cli_run().call_run(store).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let parsed = UsbDemoAppParser::parse();
    let mut app = UsbDemoApp::new(parsed.component_path)?;

    let allowed_devices = if parsed.usb_use_denylist {
        AllowedUSBDevices::Denied(parsed.usb_devices)
    } else {
        AllowedUSBDevices::Allowed(parsed.usb_devices)
    };

    let result = app.start(allowed_devices.to_owned()).await;

    dbg!(result.unwrap().unwrap());

    exit(0);
}
