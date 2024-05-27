use std::time::Duration;

use async_trait::async_trait;
use anyhow::Result;
use wasmtime::component::Resource;
use wasmtime_wasi::WasiView;

use crate::{bindings::component::usb::{types::DeviceHandleError, usb::HostDeviceHandle}, usb_host_wasi_view::USBHostWasiView};

#[derive(Debug)]
pub struct DeviceHandle {
    pub device_address: u8,
    pub handle: rusb::DeviceHandle<rusb::Context>
}

#[async_trait]
impl HostDeviceHandle for USBHostWasiView {
    fn drop(&mut self, rep: Resource<DeviceHandle>) -> Result<()>  {
        let handle = self.table().delete(rep)?;

        let was_present = self.active_device_handles.remove(&handle.device_address);
        assert!(was_present);
        Ok(())
    }

    async fn reset(&mut self, handle: Resource<DeviceHandle>) -> Result<Result<(), DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .reset()
            .map_err(|e| e.into());

        Ok(result)
    }

    async fn active_configuration(&mut self, handle: Resource<DeviceHandle>) -> Result<Result<u8, DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .active_configuration()
            .map_err(|e| e.into());

        Ok(result)
    }


    async fn select_configuration(&mut self, handle: Resource<DeviceHandle>, configuration: u8) -> Result<Result<(), DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .set_active_configuration(configuration)
            .map_err(|e| e.into());

        Ok(result)
    }

    async fn claim_interface(&mut self, handle: Resource<DeviceHandle>, interface: u8) -> Result<Result<(), DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .claim_interface(interface)
            .map_err(|e| e.into());

        Ok(result)
    }

    async fn release_interface(&mut self, handle: Resource<DeviceHandle>, interface: u8) -> Result<()> {
        let _ = self.table()
            .get_mut(&handle)?
            .handle
            .release_interface(interface)
            .map_err(|e| println!("{:?}", e));

        Ok(())
    }

    async fn write_interrupt(&mut self, handle: Resource<DeviceHandle>, endpoint: u8, data: Vec<u8>, timeout: u64) -> Result<Result<u64, DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .write_interrupt(endpoint, &data, Duration::from_nanos(timeout))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result)
    }

    async fn write_bulk(&mut self, handle: Resource<DeviceHandle>, endpoint: u8, data: Vec<u8>, timeout: u64) -> Result<Result<u64, DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .write_bulk(endpoint, &data, Duration::from_nanos(timeout))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result)
    }

    async fn write_control(&mut self, handle: Resource<DeviceHandle>, request_type: u8, request: u8, value: u16, index: u16, buf: Vec<u8>, timeout: u64) -> Result<Result<u64, DeviceHandleError>> {
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .write_control(request_type, request, value, index, &buf, Duration::from_nanos(timeout))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result)
    }

    async fn read_control(&mut self, handle: Resource<DeviceHandle>, request_type: u8, request: u8, value: u16, index: u16, max_size: u16, timeout: u64) -> Result<Result<(u64, Vec<u8>), DeviceHandleError>> {
        let mut buf: Vec<u8> = vec![0; max_size as usize];
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .read_control(request_type, request, value, index, &mut buf, Duration::from_nanos(timeout))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result.map(|bytes_read| (bytes_read, buf)))
    }

    async fn write_isochronous(&mut self, _: Resource<DeviceHandle>, _: u8, _: Vec<u8>, _: u64) -> Result<Result<u64, DeviceHandleError>> {
        todo!()
    }

    async fn read_isochronous(&mut self, _: Resource<DeviceHandle>, _: u8, _: u64) -> Result<Result<(u64, Vec<u8>), DeviceHandleError>> {
        todo!()
    }


    async fn read_bulk(&mut self, handle: Resource<DeviceHandle>, endpoint: u8, max_size: u64, timeout: u64) -> Result<Result<(u64, Vec<u8>), DeviceHandleError>> {
        let mut buffer: Vec<u8> = vec![0; max_size as usize];
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .read_bulk(endpoint, &mut buffer, Duration::from_nanos(timeout))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result.map(|a| (a, buffer)))
    }

    async fn read_interrupt(&mut self, handle: Resource<DeviceHandle>, endpoint: u8, timeout: u64) -> Result<Result<(u64, Vec<u8>), DeviceHandleError>> {
        let mut buf = [0; 256];
        let result = self.table()
            .get_mut(&handle)?
            .handle
            .read_interrupt(endpoint, &mut buf, Duration::from_nanos(timeout))
            .map_err(|e| e.into())
            .map(|a| a as u64);

        Ok(result.map(|a| (a, buf.to_vec())))
    }

    async fn select_alternate_interface(&mut self, handle: Resource<DeviceHandle>, interface: u8, setting: u8) -> Result<Result<(), DeviceHandleError>> {

        let result = self.table()
            .get_mut(&handle)?
            .handle
            .set_alternate_setting(interface, setting)
            .map_err(|e| e.into());

        Ok(result)
    }

    // async fn detach_kernel_driver(&mut self, handle: Resource<MyDeviceHandle>, interface: u8) -> Result<Result<(), DeviceHandleError>> {
    //     let result = self.table()
    //         .get_mut(&handle)?
    //         .handle
    //         .detach_kernel_driver(interface)
    //         .map_err(|e| e.into());

    //     Ok(result)
    // }

    // async fn kernel_driver_active(&mut self, handle: Resource<MyDeviceHandle>, interface: u8) -> Result<Result<bool, DeviceHandleError>> {
    //     let result = self.table()
    //         .get_mut(&handle)?
    //         .handle
    //         .kernel_driver_active(interface)
    //         .map_err(|e| e.into());

    //     Ok(result)
    // }
}
