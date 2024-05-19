use anyhow::{anyhow, Result};
use clap::Parser;
use usb_host_wasi_view::USBHostWasiView;
use wasmtime_wasi::bindings::Command;
use std::path::PathBuf;
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

    async fn start(&mut self) -> anyhow::Result<Result<(), ()>> {
        let data = USBHostWasiView::new()?;
        let mut store = Store::new(&self.engine, data);

        let (command, _) = Command::instantiate_async(&mut store, &self.component, &self.linker).await?;

        command.wasi_cli_run().call_run(store).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let parsed = UsbDemoAppParser::parse();
    let mut app = UsbDemoApp::new(parsed.component_path)?;

    app.start().await?
        .map_err(|_| anyhow!("Failed to run component."))
}
