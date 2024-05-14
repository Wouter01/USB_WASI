use std::path::Path;
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::error::TryRecvError;
use wasmtime_wasi::{DirPerms, FilePerms, ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

use crate::events;
use crate::bindings::component::usb;
use crate::bindings::component::usb::events::{Host as EventsHost, DeviceConnectionEvent as WasmDeviceConnectionEvent};

pub(crate) struct USBHostWasiView {
    table: ResourceTable,
    ctx: WasiCtx,
    updates: tokio::sync::mpsc::Receiver<events::DeviceConnectionEvent>,
    registration: rusb::Registration<rusb::Context>,
    task: tokio::task::JoinHandle<()>
}

impl USBHostWasiView {
    pub fn new() -> Result<Self> {
        let table = ResourceTable::new();

        let ctx = WasiCtxBuilder::new()
            .inherit_stdio()
            .preopened_dir(Path::new("."), ".", DirPerms::all(), FilePerms::all())?
            .build();

        let (receiver, registration, task) = events::device_connection_updates()?;
        Ok(Self { table, ctx, updates: receiver, registration, task })
    }
}

impl WasiView for USBHostWasiView {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

impl usb::usb::Host for USBHostWasiView {}
impl usb::descriptors::Host for USBHostWasiView {}
impl usb::types::Host for USBHostWasiView {}

#[async_trait]
impl EventsHost for USBHostWasiView {
    async fn update(&mut self) -> Result<WasmDeviceConnectionEvent> {
        let mapped = match self.updates.try_recv() {
            Ok(events::DeviceConnectionEvent::Connected(device)) => {
                let d = self.table().push(device)?;
                WasmDeviceConnectionEvent::Connected(d)
            },

            // TODO: Should this drop the device instead of creating a new one?
            Ok(events::DeviceConnectionEvent::Disconnected(device)) => {
                let d = self.table().push(device)?;
                WasmDeviceConnectionEvent::Disconnected(d)
            },
            Err(TryRecvError::Empty) => WasmDeviceConnectionEvent::Pending,
            Err(TryRecvError::Disconnected) => WasmDeviceConnectionEvent::Closed
        };

        Ok(mapped)
    }
}
