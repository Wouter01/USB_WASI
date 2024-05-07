mod mass_storage_device;
mod command_wrapper;

use anyhow::{Result, anyhow};

use exfat::{directory::{self, Item}, file::FileReader, ExFat};
use mass_storage_device::MassStorageDevice;
use slice::IoSlice;
use std::{fs::File, io::{BufReader, IoSliceMut, Read, Seek, Write}, time::Instant};


fn main() -> Result<()> {
    let mut device = MassStorageDevice::new()?;
    let block_length = device.capacity.block_length;

    // Read the Master Boot Record to get information about the device partitions.
    let mbr = mbrman::MBR::read_from(&mut device, block_length)?;

    // Select the first used partition.
    let data_partition = mbr
        .iter()
        .find(|p| p.1.is_used())
        .ok_or(anyhow!("No used partition found"))?
        .1;

    // Apply a slice to the device stream, so only the selected partition is considered when reading.
    let slice_start = data_partition.starting_lba * block_length;
    let slice_end = (data_partition.starting_lba + data_partition.sectors) * block_length;
    let slice = IoSlice::new(device, slice_start as u64, slice_end as u64)?;

    // Apply buffering to the stream to increase performance.
    let buffered_stream = BufReader::new(slice);
    let image = ExFat::open(buffered_stream)?;

    let now = Instant::now();

    for item in image {
        if let Item::File(mut file) = item {
            let filename = file.name().to_owned();
            let mut handle = file.open().unwrap().unwrap();
            let mut c = String::new();
            let mut data: Vec<u8> = Vec::new();
            handle.read_to_end(&mut data)?;

            println!("{} {} {:?}",filename, c.len(), data.len());
        }
    }

    println!("{:?}", now.elapsed());

    Ok(())
}
