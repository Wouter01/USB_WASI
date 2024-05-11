mod bindings;

use std::{num::Wrapping, time::{Duration, Instant}};

use anyhow::Result;
use bindings::component::usb::{device::DeviceHandle, events::update, types::{Direction, TransferType}};
use bitflags::bitflags;
use tokio::{task::AbortHandle, time::sleep};

use crate::bindings::{
    component::usb::{device::UsbDevice, events::DeviceConnectionEvent},
    Guest,
};

struct Component;

impl Guest for Component {
    #[tokio::main(flavor = "current_thread")]
    async fn run() -> Result<(), String> {

        let mut process_task_aborthandle: Option<AbortHandle> = None;

        println!("Waiting for Stadia controller...");

        loop {
            match update() {
                DeviceConnectionEvent::Pending => sleep(Duration::from_secs(1)).await,
                DeviceConnectionEvent::Closed => return Err("No further device updates.".to_string()),
                DeviceConnectionEvent::Connected(device) if device.is_stadia_device() => {
                    println!("Found Stadia Controller");

                    let (handle, endpoint_address, endpoint_out_address) = Self::setup_handle(device)?;
                    let task = tokio::spawn(async move {
                        Self::process_input_task_read_controller_state(handle, endpoint_address, endpoint_out_address).await
                        // Self::process_input_task(handle, endpoint_address, endpoint_out_address).await
                    });
                    if let Some(handle) = process_task_aborthandle {
                        handle.abort();
                    }

                    process_task_aborthandle = Some(task.abort_handle());
                },
                DeviceConnectionEvent::Disconnected(device) if device.is_stadia_device() => {
                    if let Some(handle) = process_task_aborthandle {
                        handle.abort();
                        println!("Device disconnected, stop watching.");
                        process_task_aborthandle = None;
                    }
                },
                _ => continue
            }
        }
    }
}

impl UsbDevice {
    fn is_stadia_device(&self) -> bool {
        let props = self.properties();
        props.product_id == 0x9400 && props.vendor_id == 0x18d1
    }
}

impl Component {
    fn setup_handle(device: UsbDevice) -> Result<(DeviceHandle, u8, u8), String> {
        let configurations = device
            .configurations()
            .map_err(|e| e.message())?;

        let configuration = configurations
            .first()
            .ok_or("Device has no configurations")?;

        let interface = configuration
            .interfaces
            .iter()
            .find(|i| i.number == 1)
            .ok_or("Device has no interface with number 1")?;

        let interface_descriptor = interface
            .descriptors
            .first()
            .ok_or("Interface has no descriptors")?;

        let endpoint = interface_descriptor
            .endpoint_descriptors
            .iter()
            .find(|e| e.direction == Direction::In && e.transfer_type == TransferType::Interrupt)
            .ok_or("No endpoint in interface with direction IN and transfer type Interrupt")?;

        let endpoint_out = interface_descriptor
            .endpoint_descriptors
            .iter()
            .find(|e| e.direction == Direction::Out && e.transfer_type == TransferType::Interrupt)
            .ok_or("No endpoint in interface with direction OUT and transfer type Interrupt")?;

        let handle = device
            .open()
            .map_err(|e| e.message())?;

        handle.select_configuration(configuration.number);
        handle.claim_interface(interface_descriptor.number);

        println!("Connected to controller");

        Ok((handle, endpoint.address, endpoint_out.address))
    }

    /// Repeatedly set the rumble intensity of the controller from 0 to 255.
    async fn process_input_task_write_to_controller(handle: DeviceHandle, endpoint_out_address: u8) {
        println!("Sending rumble data to controller...");

        let mut intensity = Wrapping(0u8);
        let now = Instant::now();

        for _ in 0..10000 {
            intensity += 1;
            let num = intensity.0;

            let rumble_data: [u8; 5] = [0x05, num, num, num, num];
            _ = handle.write_interrupt(endpoint_out_address, &rumble_data);
        }

        let elapsed_time = now.elapsed();
        println!("Writing data took {} milliseconds.", elapsed_time.as_millis());
    }

    /// Read and print out the controller state and react to updates.
    /// Set the rumble intensity based on the pressure on the shoulder triggers.
    async fn process_input_task_read_controller_state(handle: DeviceHandle, endpoint_address: u8, endpoint_out_address: u8) {
        println!("Waiting for controller input...");

        loop {
            let now = Instant::now();
            // Read state of controller.
            let data = handle
                .read_interrupt(endpoint_address)
                .map_err(|e| e.to_string());

            let elapsed_time = now.elapsed();
            println!("Reading state took {} milliseconds.", elapsed_time.as_millis());

            if let Ok(data) = data {
                let stadia_state = StadiaState::new(data.1);
                println!("{:?}", stadia_state);

                // Let controller vibrate at intensity of pressure of shoulder trigger.
                // When a shoulder trigger is pushed harder, the controller will rumble harder at that side.
                // Info about rumble data: https://github.com/FIX94/Nintendont/issues/1080
                let rumble_data: [u8; 5] = [0x05, stadia_state.l2_position, stadia_state.l2_position, stadia_state.r2_position, stadia_state.r2_position];
                _ = handle.write_interrupt(endpoint_out_address, &rumble_data);
            }

            tokio::task::yield_now().await;
        }
    }
}

struct StadiaState {
    dpad: DPadState,
    game_buttons: GameButtons,
    left_stick_position: (u8, u8),
    right_stick_position: (u8, u8),
    l2_position: u8,
    r2_position: u8
}

impl StadiaState {
    fn new(input: Vec<u8>) -> Self {
        Self {
            dpad: DPadState::from_bits_truncate(input[1]),
            game_buttons: GameButtons::from_bits_truncate((input[2] as u16) << 8 | input[3] as u16),
            left_stick_position: (input[4], input[5]),
            right_stick_position: (input[6], input[7]),
            l2_position: input[8],
            r2_position: input[9]
        }
    }
}

impl std::fmt::Debug for StadiaState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("
            dpad: {},
            buttons: {},
            left stick: x: {} y: {},
            right stick: x: {} y: {},
            l2: {},
            r2: {}
        ",
        defined_flags(&self.dpad),
        defined_flags(&self.game_buttons),
        self.left_stick_position.0, self.left_stick_position.1,
        self.right_stick_position.0, self.right_stick_position.1,
        self.l2_position,
        self.r2_position
        ))
    }
}

fn defined_flags<F: bitflags::Flags>(value: &F) -> String {
    value.iter_names().map(|i| i.0).collect::<Vec<_>>().join("|")
}

bitflags! {
    struct DPadState: u8 {
        const left = 0b110;
        const right = 0b10;
        const up = 0b0;
        const down = 0b100;
        const left_up = 0b111;
        const right_up = 0b1;
        const left_down = 0b101;
        const right_down = 0b11;
    }

    struct GameButtons: u16 {
        const left_stick_button  = 0b1;
        const r1                 = 0b10;
        const l1                 = 0b100;
        const y                  = 0b1000;
        const x                  = 0b10000;
        const b                  = 0b100000;
        const a                  = 0b1000000;
        const screenshot_button  = 0b1 << 8;
        const assistant_button   = 0b10 << 8;
        const l2_button          = 0b100 << 8;
        const r2_button          = 0b1000 << 8;
        const stadia_button      = 0b10000 << 8;
        const menu_button        = 0b100000 << 8;
        const options_button     = 0b1000000 << 8;
        const right_stick_button = 0b10000000 << 8;
    }
}

fn main() {
    println!("Call run instead");
}
