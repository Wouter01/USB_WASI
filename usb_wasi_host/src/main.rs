
use wasmtime::{component::*, Config, Engine, Store};
use wasmtime_wasi::preview2::{command, WasiCtx, WasiCtxBuilder, WasiView, Table};
use async_trait::async_trait;
use clap::Parser;
use std::path::PathBuf;

use crate::bindings::component::usb::device::{HostUsbDevice, Properties, UsbDevice};
use crate::bindings::component::usb::types::{Version};
use crate::bindings::Usb;

pub mod bindings {
    wasmtime::component::bindgen!({
        world: "component:usb/usb",
        async: true,
        with: {
            "component:usb/device/usb-device": super::MyDevice,
        }
    });
}

// Implement the From trait for conversion from A to B
impl From<rusb::Version> for Version {
    fn from(a: rusb::Version) -> Self {
        Version { major: a.0, minor: a.1, subminor: a.2 }
    }
}

// Implement the From trait for conversion from B to A
impl From<Version> for rusb::Version {
    fn from(b: Version) -> Self {
        rusb::Version { 0: b.major, 1: b.minor, 2: b.subminor }
    }
}

struct ServerWasiView {
    table: Table,
    ctx: WasiCtx
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

#[derive(Debug)]
pub struct MyDevice {
    device: rusb::Device<rusb::GlobalContext>
}

impl MyDevice {
    fn get_properties(&self) -> anyhow::Result<Properties> {
        let descriptor = self.device.device_descriptor()?;
        
        let props = Properties {
            device_class: descriptor.class_code(),
            device_protocol: descriptor.protocol_code(),
            device_subclass: descriptor.sub_class_code(),
            device_version: descriptor.device_version().into(),
            product_id: descriptor.product_id(),
            usb_version: descriptor.usb_version().into(),
            vendor_id: descriptor.vendor_id()
        };
        
        Ok(props)
    }
}

impl bindings::component::usb::types::Host for ServerWasiView {
    
}

#[async_trait]
impl HostUsbDevice for ServerWasiView {
    
    fn drop(&mut self, rep: Resource<UsbDevice>) -> wasmtime::Result<()> {
        Ok(self
        .table_mut()
        .delete(rep)
        .map(|_| ())?)
    }
    
    async fn properties(&mut self, rep: Resource<UsbDevice>) -> wasmtime::Result<Properties> {
        self.table().get(&rep)?.get_properties()
    }
}

#[async_trait]
impl bindings::component::usb::device::Host for ServerWasiView {
    async fn get_devices(&mut self,) -> wasmtime::Result<Vec<wasmtime::component::Resource<UsbDevice>>> {
        rusb::devices()?
        .iter()
        .map(|device| {
           self
           .table_mut()
           .push(MyDevice { device })
           .map_err(wasmtime::Error::from)
        })
        .collect()
    }
}


#[derive(Parser)]
#[clap(name = "usb", version = env!("CARGO_PKG_VERSION"))]
struct UsbDemoApp {
    /// The path to the guest component.
    #[clap(value_name = "COMPONENT_PATH")]
    component: PathBuf,
}

impl UsbDemoApp {
    async fn run(self) -> anyhow::Result<()> {
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
        
        let run = instance.get_typed_func::<(), (u32,)>(&mut store, "hello")?;
        
        run.call_async(&mut store, ()).await?;
        
        Ok(())
    }
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    UsbDemoApp::parse().run().await
}
