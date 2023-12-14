use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use wasmtime::{component::*, Config, Engine, Store};
use wasmtime_wasi::preview2::{command, Table, WasiCtx, WasiCtxBuilder, WasiView};

use crate::bindings::Usb;

pub mod conversion;
pub mod device;
pub use device::usbdevice::MyDevice;

pub mod bindings {
    wasmtime::component::bindgen!({
        world: "component:usb/usb",
        async: true,
        with: {
            "component:usb/device/usb-device": super::MyDevice,
        }
    });
}

struct ServerWasiView {
    table: Table,
    ctx: WasiCtx,
}

impl ServerWasiView {
    fn new() -> Self {
        let table = Table::new();
        let ctx = WasiCtxBuilder::new().inherit_stdio().build();
        Self { table, ctx }
    }
}

impl WasiView for ServerWasiView {
    fn table(&self) -> &Table {
        &self.table
    }

    fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }

    fn ctx(&self) -> &WasiCtx {
        &self.ctx
    }

    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

impl bindings::component::usb::types::Host for ServerWasiView {}

#[derive(Parser)]
#[clap(name = "usb", version = env!("CARGO_PKG_VERSION"))]
struct UsbDemoApp {
    /// The path to the guest component.
    #[clap(value_name = "COMPONENT_PATH")]
    component: PathBuf,
}

impl UsbDemoApp {
    async fn run(self) -> Result<()> {
        let mut config = Config::default();
        config.wasm_component_model(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);

        let data = ServerWasiView::new();

        let mut store = Store::new(&engine, data);

        command::add_to_linker(&mut linker)?;

        let component = Component::from_file(&engine, self.component)?;

        Usb::add_to_linker(&mut linker, |view| view)?;

        let instance = linker.instantiate_async(&mut store, &component).await?;

        let run = instance.get_typed_func::<(), ()>(&mut store, "hello")?;

        run.call_async(&mut store, ()).await?;

        Ok(())
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    UsbDemoApp::parse().run().await
}
