
use wasmtime::{component::*, Config, Engine, Store};
use wasmtime_wasi::preview2::{command, Table, WasiCtx, WasiCtxBuilder, WasiView};
// use wasmtime::component::ResourceTable;
use async_trait::async_trait;
use std::error::Error;

use crate::bindings::component::usb::device::{HostUsbDevice, Properties, UsbDevice};
use crate::bindings::Usb;
// use example::service::{
//     logging::{self, HostLogger},
//     types::{self, HostRequest, HostResponse},
// };
// use exports::example::service::handler;
pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "component:usb/usb",
        async: true
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

#[derive(Debug)]
struct MyDevice {
    init: Vec<u8>,
    device: rusb::Device<rusb::GlobalContext>
}

#[async_trait]
impl HostUsbDevice for ServerWasiView {
    // async fn new(&mut self, init:Vec<u8> ,) -> anyhow::Result<Resource<bindings::component::usb::device::UsbDevice>> {
    //     println!("New Device");
    //     let request = MyDevice { init };
    //     let something = self.table_mut()
    //     .push(request)?;
    //     
    //     Ok(
    //        Resource::new_own(something.rep())
    //     )
    // }
    
    async fn test(&mut self, self_: Resource<UsbDevice> ,) -> wasmtime::Result<String> {
        Ok("kiekeboe".to_string())
    }
    
    fn drop(&mut self, rep: wasmtime::component::Resource<UsbDevice>) -> wasmtime::Result<()> {
        println!("Drop Resource");
    //     Ok(self
    //     .table_mut()
    //     .delete(rep)
    //     .map(|_| ())?)
        Ok(())
    }
    
    async fn properties(&mut self, rep: wasmtime::component::Resource<bindings::component::usb::device::UsbDevice>) -> wasmtime::Result<Properties> {
        println!("Getting properties...");
        
        let device = self.table_mut().get_any_mut(rep.rep())?;
        let usbdevice: Option<&mut MyDevice> = device.downcast_mut();
        
        if let Some(device) = usbdevice {
            println!("deviceeee: {:?}", device);
        } else {
            println!("Could not cast");
        }
        
        Ok(Properties { device_class: 0 })
    }
}

pub struct DeviceFilter {
    vendor_id: Option<u16>,
    product_id: Option<u16>,
    serial_number: Option<String>,
    class_code: String,
    subclass_code: String,
    protocol_code: String
}

impl DeviceFilter {
    fn matches_device(&self, device: &rusb::Device<rusb::GlobalContext>) -> bool {
        return true;
        let descriptor = {
            let descriptor = device.device_descriptor();
            match descriptor {
                Ok(x) => x,
                Err(_) => return false
            }
        };
        
        if let Some(vendor_id) = self.vendor_id {
            if vendor_id != descriptor.vendor_id() {
                return false;
            }
        }
        
        if let Some(product_id) = self.product_id {
            if product_id != descriptor.product_id() {
                return false;
            }
        }
        
        if let Some(serial_number) = self.serial_number.as_deref() {
            let handle = device.open().unwrap();
            let sn_res = handle.read_serial_number_string_ascii(&descriptor);
            let sn = match sn_res {
                Ok(x) => x,
                Err(_) => return false
            };
            if serial_number != sn {
                return false;
            }
        }
        
        return true;
    }
}

pub fn get_device_list(filters: Vec<DeviceFilter>) -> Vec<rusb::Device<rusb::GlobalContext>> {
    println!("Getting Device List...");
    let devices = rusb::devices();
    if let Ok(devices) = devices {
        devices
            .iter()
            .filter_map(|device| {
                println!("Device: {:?}", device);
                if filters.iter().find(|d| d.matches_device(&device)).is_some() {
                    return Some(device);
                }

                return None;
            })
            .collect()
    } else {
        vec![]
    }
}

use futures::future::join_all;

#[async_trait]
impl bindings::component::usb::device::Host for ServerWasiView {
    async fn get_devices(&mut self,) ->  wasmtime::Result<Vec<wasmtime::component::Resource<UsbDevice>>> {
        let filter = DeviceFilter { 
            product_id: Some(0),
            serial_number: Some("".to_string()),
            vendor_id: Some(0),
            class_code: "".to_string(),
             subclass_code: "".to_string(), 
             protocol_code: "".to_string()
         };
        let devices = get_device_list(vec![filter]);
        println!("Mapped Devices: {:?}", devices);

        let mut hosts: Vec<anyhow::Result<Resource<UsbDevice>>> = vec![];
        for device in devices {
            let request = MyDevice { init: vec![device.address()], device: device };
            let something = self.table_mut()
            .push(request)?;
            
            
            hosts.push(Ok(Resource::new_own(something.rep())));
        }
        hosts.into_iter().collect()
    }
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    
    let mut config = Config::default();
    config.wasm_component_model(true);
    config.async_support(true);
    
    let engine = Engine::new(&config)?;
    
    // For host-provided functions it's recommended to use a `Linker` which does
    // name-based resolution of functions.
    let mut linker = Linker::new(&engine);
    
    let data = ServerWasiView::new();
    
    let mut store = Store::new(&engine, data);
    
    command::add_to_linker(&mut linker)?;
    
    let component = Component::from_file(&engine, "/Volumes/Macintosh HD/Users/wouter/Developer/masterproef/USB_WASI/target/wasm32-wasi/debug/usb-component-wasi-guest.wasm")?;
    
    Usb::add_to_linker(&mut linker, |view| view)?;
    
    let instance = linker.instantiate_async(&mut store, &component).await?;
    
    
    // println!("Exports: {:?}", instance.exports(&store).root());
    // Like before, we can get the run function and execute it.

    let run = instance.get_typed_func::<(), (u32,)>(&mut store, "hello")?;
    
    println!("Readi0");
    let res2: (u32,) = run.call_async(&mut store, ()).await?;
    
    // We can also inspect what integers were logged:
    println!("logged integers {:?}", res2);
    
    Ok(())
    // Parse the command line arguments and run the application
    // ServerApp::parse().run()
}
