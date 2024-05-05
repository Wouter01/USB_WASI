use bitflags::bitflags;


pub struct CommandBlockWrapper {
    tag: u32,
    data_transfer_length: u32,
    direction: CBWDirection,
    lun: u8,
    cbwcb: Vec<u8>
}

enum CBWDirection {
    In = 0b1000_0000,
    Out = 0b0000_0000
}

pub struct CommandStatusWrapper {
    tag: u32,
    data_residue: u32,
    status: CSWStatus
}

enum CSWStatus {
    Passed = 0x00,
    Failed = 0x01,
    Error = 0x02
}
