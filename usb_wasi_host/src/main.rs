use anyhow::Result;
use clap::Parser;
use tokio::sync::mpsc::error::TryRecvError;
use std::path::PathBuf;
use wasmtime::{component::*, Config, Engine, Store};
use wasmtime_wasi::preview2::{command, Table, WasiCtx, WasiCtxBuilder, WasiView};
use async_trait::async_trait;

use bindings::component::usb::events::{Host as EventsHost, DeviceConnectionEvent as WasmDeviceConnectionEvent};

use crate::bindings::UsbHost;

pub mod conversion;
pub mod device;
pub use device::usbdevice::MyDevice;
mod events;

pub type GlobalUsbDevice = MyDevice<rusb::Context>;

pub mod bindings {
    wasmtime::component::bindgen!({
        world: "component:usb/usb-host",
        async: true,
        with: {
            "component:usb/device/usb-device": super::GlobalUsbDevice,
        }
    });
}

#[allow(dead_code)]
struct ServerWasiView {
    table: Table,
    ctx: WasiCtx,
    updates: tokio::sync::mpsc::Receiver<events::DeviceConnectionEvent>,
    registration: rusb::Registration<rusb::Context>,
    task: tokio::task::JoinHandle<()>
}

impl ServerWasiView {
    fn new() -> Result<Self> {
        let table = Table::new();
        let ctx = WasiCtxBuilder::new().inherit_stdio().build();
        let (receiver, registration, task) = events::device_connection_updates()?;
        Ok(Self { table, ctx, updates: receiver, registration, task })
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

#[async_trait]
impl EventsHost for ServerWasiView {
    async fn update(&mut self) -> Result<WasmDeviceConnectionEvent> {
        let mapped = match self.updates.try_recv() {
            Ok(events::DeviceConnectionEvent::Connected(device)) => {
                let d = self.table_mut().push(device)?;
                WasmDeviceConnectionEvent::Connected(d)
            },
            
            // TODO: Should this drop the device instead of creating a new one?
            Ok(events::DeviceConnectionEvent::Disconnected(device)) => {
                let d = self.table_mut().push(device)?;
                WasmDeviceConnectionEvent::Disconnected(d)
            },
            Err(TryRecvError::Empty) => WasmDeviceConnectionEvent::Pending,
            Err(TryRecvError::Disconnected) => WasmDeviceConnectionEvent::Closed
        };
        
        Ok(mapped)
    }
}

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
    component: Component
}

impl UsbDemoApp {
    async fn create(component: PathBuf) -> Result<Self> {
        
        let mut config = Config::default();
        config.wasm_component_model(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);

        command::add_to_linker(&mut linker)?;
        let component = Component::from_file(&engine, component)?;

        UsbHost::add_to_linker(&mut linker, |view| view)?;
        
        let runner = Runner {
            engine,
            linker,
            component
        };
        
        Ok(Self { runner })
    }
}

async fn start_guest(runner: &mut Runner) -> Result<()> {
    let data = ServerWasiView::new()?;
    let mut store = Store::new(&runner.engine, data);
    
    let instance = &runner.linker.instantiate_async(&mut store, &runner.component).await?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run").unwrap();
    
    run.call_async(&mut store, ()).await
}

#[tokio::main]
async fn main() -> Result<()> {
    let parsed = UsbDemoAppParser::parse();
    
    let mut app = UsbDemoApp::create(parsed.component).await?;
    let runner = &mut app.runner;
    
    // let mut stream = events::device_connection_updates();
    // while let Some(message) = stream.recv().await {
    //     println!("Received: {:?}", message);
    //     
    // }
    let _ = start_guest(runner).await;
    
    println!("Guest Ended");
    
    Ok(())
}
