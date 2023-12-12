
use wasmtime::{component::*, Config, Engine, Store};
use wasmtime_wasi::preview2::{command, WasiCtx, WasiCtxBuilder, WasiView, Table};
use async_trait::async_trait;
use std::collections::HashMap;


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

pub struct State {
    devices: HashMap<u32, MyDevice>,
    
    current_id: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            devices: HashMap::new(),
            current_id: 0
        }
    }
}
struct ServerWasiView {
    table: Table,
    ctx: WasiCtx,
    state: State
}

impl State {
    fn devices_mut(&mut self) -> &mut HashMap<u32, MyDevice> {
        &mut self.devices
    }
}

impl ServerWasiView {
    fn new() -> Self {
        let table = Table::new();
        let ctx = WasiCtxBuilder::new().inherit_stdio().build();
        let state = State::default();
        Self { table, ctx, state }
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

impl State {
    pub fn new_id(&mut self) -> u32 {
        self.current_id += 1;
        self.current_id
    }
}

// /// A result of a function that may return a `Error`.
// pub type Result<T> = result::Result<T, Error>;
// 
// #[derive(Debug, Copy, Clone, Eq, PartialEq)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// pub enum Error {
//     Descriptor(rusb::Error)
// }


impl MyDevice {
    fn get_properties(self) -> anyhow::Result<Properties> {
        let descriptor = self.device.device_descriptor()?;
        
        let props = Properties {
            device_class: descriptor.class_code()
        };
        
        Ok(props)
    }
}
// impl<T: ServerWasiView> HostUsbDevice for T {
//     fn drop(&mut self, fields: Resource<HostUsbDevice>) -> wasmtime::Result<()> {
//         
//     }
// }

impl bindings::component::usb::types::Host for ServerWasiView {
    
}

enum MyError {
    Unknown
}

#[async_trait]
impl HostUsbDevice for ServerWasiView {
    
    fn drop(&mut self, rep: Resource<UsbDevice>) -> wasmtime::Result<()> {
        println!("Drop Resource");
        // Resource
        Ok(self
        .state
        .devices_mut()
        .remove(&rep.rep())
        // .delete(rep.rep())
        .map(|_| ()).ok_or(wasmtime::Error::msg("Test"))?)
        // Ok(())
    }
    
    async fn properties(&mut self, rep: Resource<UsbDevice>) -> wasmtime::Result<Properties> {
        println!("Getting properties...");
        let usbdevice = self.state.devices.get(&rep.rep());
        // let device = self.table_mut().get_any_mut(rep.rep())?;
        // let usbdevice: Option<&mut MyDevice> = device.downcast_mut();
        
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

#[async_trait]
impl bindings::component::usb::device::Host for ServerWasiView {
    async fn get_devices(&mut self,) -> wasmtime::Result<Vec<wasmtime::component::Resource<UsbDevice>>> {
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
            
            let id = self.state.new_id();
            let something = self.state.devices.insert(id, request);
            
            
            hosts.push(Ok(Resource::new_own(id)));
        }
        hosts.into_iter().collect()
    }
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    
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
