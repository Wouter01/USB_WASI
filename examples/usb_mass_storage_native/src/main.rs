mod mass_storage_device;
mod command_wrapper;

use anyhow::Result;

use exfat::{directory::{self, Item}, ExFat};
use mass_storage_device::MassStorageDevice;
use std::{fs::File, io::{Read, Seek}};

fn main() -> Result<()> {

    let device = MassStorageDevice::new()?;

    println!("{:?}", device.max_lun());


    let image = ExFat::open(device).expect("cannot open exFAT image from exfat.img");

    for item in image {
        let name = match item {
            Item::File(file) => file.name().to_owned(),
            Item::Directory(directory) => directory.name().to_owned()
        };
        // item will be either file or directory.
    }

    Ok(())
}
