mod mass_storage_device;
mod command_wrapper;

#[cfg(target_arch = "wasm32")]
mod bindings;

use anyhow::{Result, anyhow};

use exfat::{directory::{self, Item}, file::FileReader, ExFat};
use mass_storage_device::MassStorageDevice;
use slice::IoSlice;
use std::{fs::File, hash::{DefaultHasher, Hash, Hasher}, io::{BufReader, IoSliceMut, Read, Seek, Write}, ops::{Add, AddAssign}, thread::sleep, time::{Duration, Instant}};

fn main() -> Result<()> {

    let mut device = MassStorageDevice::new()?;

    let mut success = 0;
    for _ in 0..1000 {
        success += if test(&mut device).is_ok() { 1 } else { 0 };
    }

    dbg!(success);

    Ok(())
}

fn test(device: &mut MassStorageDevice) -> Result<()> {

    let block_length = device.capacity.block_length;

    // Read the Master Boot Record to get information about the device partitions.
    let mbr = mbrman::MBR::read_from(device, block_length)?;

    // Select the first used partition.
    let data_partition = mbr
        .iter()
        .find(|p| p.1.is_used())
        .ok_or(anyhow!("No used partition found"))?
        .1;

    // Apply a slice to the device stream, so only the selected partition is considered when reading.
    let slice_start = data_partition.starting_lba * block_length;
    let slice_end = (data_partition.starting_lba + data_partition.sectors) as u64 * block_length as u64;
    let slice = IoSlice::new(device, slice_start as u64, slice_end as u64)?;

    // Apply buffering to the stream to increase performance.
    let buffered_stream = BufReader::new(slice);
    let image = ExFat::open(buffered_stream)?;

    let now = Instant::now();

    for item in image {
        read_item(item)?;
    }

    println!("{:?}", now.elapsed().as_millis());

    Ok(())
}

fn read_item<T: Seek + Read>(item: Item<T>) -> Result<()> {
    match item {
        Item::File(mut file) => {
            let filename = file.name().to_owned();
            let handle = file.open();
            // file_count.add_assign(1);

            match handle {
                Ok(Some(mut handle)) => {
                    let mut data: Vec<u8> = Vec::new();
                    handle.read_to_end(&mut data)?;
                    // dbg!(filename, data.len());
                },

                Ok(None) => {},

                Err(e) => {}
            }

        }

        Item::Directory(directory) => {
            let items = directory.open()?;
            for item in items {
                read_item(item)?;
            }
        }

        _ => {}
    }

    Ok(())
}
