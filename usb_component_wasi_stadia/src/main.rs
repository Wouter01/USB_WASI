mod bindings;

use std::{ thread::sleep, time::Duration};

use anyhow::Result;
use bindings::component::usb::{events::update, types::{Direction, TransferType}};
use bitflags::bitflags;

use crate::bindings::{
    component::usb::{device::{get_devices, UsbDevice}, events::DeviceConnectionEvent},
    Guest,
};

struct Component;

impl Guest for Component {
    #[tokio::main(flavor = "current_thread")]
    async fn run() -> Result<(), String> {
        let stadia_device: UsbDevice;
        loop {
            match update() {
                DeviceConnectionEvent::Pending => {
                    sleep(Duration::from_secs(1));
                },
                DeviceConnectionEvent::Closed => return Err("No further device updates.".to_string()),
                DeviceConnectionEvent::Connected(device) => {
                    let props = device.properties();
                    let is_stadia = props.product_id == 0x9400 && props.vendor_id == 0x18d1;
                    if is_stadia {
                        println!("Found Stadia Controller");
                        stadia_device = device;
                        break;
                    }
                },
                DeviceConnectionEvent::Disconnected(_) => continue,
            }
        }

        let configurations = stadia_device
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

        let handle = stadia_device
            .open()
            .map_err(|e| e.message())?;

        handle.select_configuration(configuration.number);
        handle.claim_interface(interface_descriptor.number);

        println!("Connected to controller");
        println!("Waiting for controller input...");

        loop {
            let data = handle
                .read_interrupt(endpoint.address)
                .map_err(|e| e.to_string());

            if let Ok(data) = data {
                let stadia_state = StadiaState::new(data.1);
                println!("{:?}", stadia_state);
            }
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
