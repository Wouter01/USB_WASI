use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use wasmtime::{component::*, Config, Engine, Store};
use wasmtime_wasi::preview2::{command, Table, WasiCtx, WasiCtxBuilder, WasiView};

use crate::bindings::UsbHost;

pub mod conversion;
pub mod device;
pub use device::usbdevice::MyDevice;
mod events;

pub type GlobalUsbDevice = MyDevice<rusb::GlobalContext>;

pub mod bindings {
    wasmtime::component::bindgen!({
        world: "component:usb/usb-host",
        async: true,
        with: {
            "component:usb/device/usb-device": super::GlobalUsbDevice,
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
struct UsbDemoAppParser {
    /// The path to the guest component.
    #[clap(value_name = "COMPONENT_PATH")]
    component: PathBuf,
}

struct UsbDemoApp {
    runner: Runner
}

#[allow(dead_code)]
struct Runner {
    engine: Engine,
    linker: Linker<ServerWasiView>,
    instance: Instance,
    component: Component,
    store: Store<ServerWasiView>,
    run: TypedFunc<(), ()>
}

impl UsbDemoApp {
    async fn create(component: PathBuf) -> Result<Self> {
        
        let mut config = Config::default();
        config.wasm_component_model(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);

        let data = ServerWasiView::new();

        command::add_to_linker(&mut linker)?;
        let component = Component::from_file(&engine, component)?;

        UsbHost::add_to_linker(&mut linker, |view| view)?;
        
        let mut store = Store::new(&engine, data);

        let instance = linker.instantiate_async(&mut store, &component).await?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "hello")?;
        
        let runner = Runner {
            engine,
            linker,
            instance,
            component,
            store,
            run
        };
        
        Ok(Self { runner })
    }
}

async fn start_guest(runner: &mut Runner) -> Result<()> {
    let data = ServerWasiView::new();
    let mut store = Store::new(&runner.engine, data);
    let component = runner.component.clone();
    let linker = runner.linker.clone();
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "hello").unwrap();
    
    let _ = run.call_async(&mut store, ()).await;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let parsed = UsbDemoAppParser::parse();
    
    let mut app = UsbDemoApp::create(parsed.component).await?;
    let runner = &mut app.runner;
    
    let mut stream = events::device_connection_updates();
    while let Some(message) = stream.recv().await {
        println!("Received: {:?}", message);
        start_guest(runner).await;
    }
    
    Ok(())
}
