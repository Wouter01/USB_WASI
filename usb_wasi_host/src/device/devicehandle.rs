use std::time::Duration;

use async_trait::async_trait;
use anyhow::Result;
use wasmtime::component::Resource;
use wasmtime_wasi::preview2::WasiView;

use crate::bindings::component::usb::device::{DeviceHandleError, HostDeviceHandle};

#[derive(Debug)]
pub struct MyDeviceHandle {
    pub handle: rusb::DeviceHandle<rusb::Context>
}

#[async_trait]
impl<T> HostDeviceHandle for T
where
    T: WasiView
{
    fn drop(&mut self, rep: Resource<MyDeviceHandle>) -> Result<()>  {
        Ok(self.table().delete(rep).map(|_| ())?)
    }

    async fn select_configuration(&mut self, handle: Resource<MyDeviceHandle>, configuration: u8) -> Result<()> {
        let _ = self.table()
            .get_mut(&handle)?
            .handle
            .set_active_configuration(configuration)
            .map_err(|e| println!("{:?}", e));

        Ok(())
    }

    async fn claim_interface(&mut self, handle: Resource<MyDeviceHandle>, interface: u8) -> Result<()> {
        let _ = self.table()
            .get_mut(&handle)?
            .handle
            .claim_interface(interface)
            .map_err(|e| println!("{:?}", e));

        Ok(())
    }

    async fn unclaim_interface(&mut self, handle: Resource<MyDeviceHandle>, interface: u8) -> Result<()> {
        let _ = self.table()
            .get_mut(&handle)?
            .handle
            .release_interface(interface)
            .map_err(|e| println!("{:?}", e));

        Ok(())
    }

    async fn write_interrupt(&mut self, handle: Resource<MyDeviceHandle>, endpoint: u8, data: Vec<u8>) -> Result<Result<u64, DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .write_interrupt(endpoint, &data, Duration::from_millis(10000))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result)
    }

    async fn write_bulk(&mut self, handle: Resource<MyDeviceHandle>, endpoint: u8, data: Vec<u8>) -> Result<Result<u64, DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .write_bulk(endpoint, &data, Duration::from_millis(10000))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result)
    }

    async fn read_bulk(&mut self, handle: Resource<MyDeviceHandle>, endpoint: u8) -> Result<Result<(u64, Vec<u8>), DeviceHandleError>> {
        let mut data = Vec::new();
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .read_bulk(endpoint, &mut data, Duration::from_millis(10))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result.map(|a| (a, data)))
    }

    async fn read_interrupt(&mut self, handle: Resource<MyDeviceHandle>, endpoint: u8) -> Result<Result<(u64, Vec<u8>), DeviceHandleError>> {
        let mut buf = [0; 256];
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .read_interrupt(endpoint, &mut buf, Duration::from_millis(10))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result.map(|a| (a, buf.to_vec())))
    }

    async fn select_alternate_interface(&mut self, handle: Resource<MyDeviceHandle>, interface: u8, setting: u8) -> Result<Result<(), DeviceHandleError>> {

        let result = self.table()
            .get_mut(&handle)?
            .handle
            .set_alternate_setting(interface, setting)
            .map_err(|e| e.into());

        Ok(result)
    }

    async fn detach_kernel_driver(&mut self, handle: Resource<MyDeviceHandle>, interface: u8) -> Result<Result<(), DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .detach_kernel_driver(interface)
            .map_err(|e| e.into());

        Ok(result)
    }

    async fn kernel_driver_active(&mut self, handle: Resource<MyDeviceHandle>, interface: u8) -> Result<Result<bool, DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .kernel_driver_active(interface)
            .map_err(|e| e.into());

        Ok(result)
    }
}
