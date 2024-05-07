use bytes::{Buf, BufMut, Bytes, BytesMut};

const CBW_SIGNATURE: u32 = 0x43425355;
const CSW_SIGNATURE: u32 = 0x53425355;

pub enum CBWDirection {
    Out = 0b0000_0000,
    In = 0b1000_0000
}

pub struct CommandBlockWrapper {
    pub tag: u32,
    pub data_transfer_length: u32,
    pub direction: CBWDirection,
    pub lun: u8,
    pub cbwcb: Vec<u8>
}

impl Into<Vec<u8>> for CommandBlockWrapper {
    fn into(self) -> Vec<u8> {
        // Check if upper bytes of lun are not used. (These are reserved)
        assert!(self.lun & 0b11110000 == 0);

        let cbwcb_len = self.cbwcb.len() as u8;
        // Check if upper 3 bytes are not used. (These are reserved)
        assert!(cbwcb_len & 0b11100000 == 0);

        let mut bytes = BytesMut::with_capacity(31);
        bytes.put_u32_le(CBW_SIGNATURE);
        bytes.put_u32_le(self.tag);
        bytes.put_u32_le(self.data_transfer_length);
        bytes.put_u8(self.direction as u8);
        bytes.put_u8(self.lun);
        bytes.put_u8(cbwcb_len.to_owned());
        bytes.put_slice(&self.cbwcb);
        bytes.put_bytes(0, 31 - bytes.remaining());
        bytes.to_vec()
    }
}

#[derive(PartialEq)]
pub enum CSWStatus {
    Passed = 0x00,
    Failed = 0x01,
    Error = 0x02
}

impl From<u8> for CSWStatus {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Passed,
            0x01 => Self::Failed,
            0x02 => Self::Error,
            _ => panic!("Unknown value")
        }
    }
}

pub struct CommandStatusWrapper {
    pub tag: u32,
    pub data_residue: u32,
    pub status: CSWStatus
}

impl From<Bytes> for CommandStatusWrapper {
    fn from(mut value: Bytes) -> Self {
        assert!(CSW_SIGNATURE == value.get_u32_le());
        Self {
            tag: value.get_u32_le(),
            data_residue: value.get_u32_le(),
            status: CSWStatus::from(value.get_u8()),
        }
    }
}

#[derive(Debug, Default)]
pub struct DeviceCapacity {
    pub logical_block_address: u32,
    pub block_length: u32,
    pub size: u64
}

impl From<Bytes> for DeviceCapacity {
    fn from(mut value: Bytes) -> Self {
        let logical_block_address = value.get_u32();
        let block_length = value.get_u32();
        Self {
            logical_block_address,
            block_length,
            size: logical_block_address as u64 * block_length as u64
        }
    }
}
