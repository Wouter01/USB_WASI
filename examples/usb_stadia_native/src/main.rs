use std::{num::Wrapping, time::{Duration, Instant}};

use anyhow::{anyhow, Result};
use bitflags::bitflags;
use tokio::{task::AbortHandle, time::sleep};

use rusb::*;

// fn read_property<S>(&self, perform: impl FnOnce(&rusb::Device<T>, DeviceHandle<T>, &Language) -> Result<S, rusb::Error>) -> Result<S, DeviceHandleError> {
//     let device = &self.device;
//     let handle = device.open().map_err(DeviceHandleError::from)?;
//     let languages = handle.read_languages(DEFAULT_TIMEOUT).map_err(DeviceHandleError::from)?;
//     let language = languages
//         .first()
//         .ok_or(DeviceHandleError::Other)?;

//     perform(device, handle, language)
//         .map_err(DeviceHandleError::from)
// }

fn main() -> anyhow::Result<()> {
    let context = rusb::Context::new();

    let device = context?
        .devices()?
        .iter()
        .find(|d| {
            let Ok(descriptor) = d.device_descriptor() else { return false };
            descriptor.product_id() == 0x9400 && descriptor.vendor_id() == 0x18d1
        })
        .ok_or(anyhow!("Could not find Stadia device"))?;

    let configuration = device.config_descriptor(0)?;

    let interface = configuration
        .interfaces()
        .find(|i| i.number() == 1)
        .ok_or(anyhow!("Interface not found"))?;

    let descriptor = interface
        .descriptors()
        .find(|_| true)
        .ok_or(anyhow!("Descriptor not found"))?;

    let endpoint = descriptor
        .endpoint_descriptors()
        .find(|e| e.direction() == Direction::In && e.transfer_type() == TransferType::Interrupt)
        .ok_or(anyhow!("No endpoint in interface with direction IN and transfer type Interrupt"))?;

    let endpoint_out = descriptor
        .endpoint_descriptors()
        .find(|e| e.direction() == Direction::Out && e.transfer_type() == TransferType::Interrupt)
        .ok_or(anyhow!("No endpoint in interface with direction IN and transfer type Interrupt"))?;

    let mut handle = device.open()?;

    handle.set_auto_detach_kernel_driver(true)?;
    handle.set_active_configuration(configuration.number())?;
    handle.claim_interface(descriptor.interface_number())?;

    let mut intensity = Wrapping(0u8);
    let address = endpoint_out.address();
    const DURATION: Duration = Duration::from_secs(10);

    let now = Instant::now();

    for _ in 0..10000 {
        intensity += 1;
        let num = intensity.0;

        let rumble_data: [u8; 5] = [0x05, num, num, num, num];
        _ = handle.write_interrupt(address, &rumble_data, DURATION);


        // let mut buffer: Vec<u8> = vec![0; endpoint.max_packet_size() as usize];
        // // Read state of controller.
        // let bytes_read = handle
        //     .read_interrupt(endpoint.address(), &mut buffer, Duration::from_secs(10))?;



        // if let Ok(data) = data {
            // let stadia_state = StadiaState::new(buffer);
            // println!("{:?}", stadia_state);

            // Let controller vibrate at intensity of pressure of shoulder trigger.
            // When a shoulder trigger is pushed harder, the controller will rumble harder at that side.
            // Info about rumble data: https://github.com/FIX94/Nintendont/issues/1080

        // }
    }

    let elapsed_time = now.elapsed();
    println!("Reading state took {} milliseconds.", elapsed_time.as_millis());

    Ok(())
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
