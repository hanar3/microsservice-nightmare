use deku::prelude::*;

#[derive(Debug, DekuRead, DekuWrite)]
pub struct PacketHeader {
    pub id: u8,
    pub data_size: u8,
}

#[derive(Debug, DekuRead, DekuWrite)]
pub struct Message {
    pub id: u8,
    pub data_size: u8,
    #[deku(count = "data_size")]
    pub data: Vec<u8>,
}

#[derive(Debug, DekuRead, DekuWrite)]
pub struct Service {
    pub name_len: u8,
    #[deku(count = "name_len")]
    pub svc_name: Vec<u8>,

    pub svc_type: u8,

    pub svc_path_len: u8,
    #[deku(count = "svc_path_len")]
    pub svc_path: Vec<u8>,

    pub cmd_len: u8,
    #[deku(count = "cmd_len")]
    pub cmd: Vec<u8>,

    pub cmd_args_len: u8,
    #[deku(count = "cmd_args_len")]
    pub cmd_args: Vec<u8>,

    pub svc_port: u16,
}

pub enum PacketId {
    AttachService = 0x1,
    DetachService = 0x2,
    LuaServices = 0x3,
}

#[derive(Debug, DekuRead, DekuWrite)]
pub struct LuaServices {
    pub filepath_len: u8,
    #[deku(count = "filepath_len")]
    pub filepath: Vec<u8>,
}

impl TryFrom<u8> for PacketId {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x1 => Ok(PacketId::AttachService),
            0x2 => Ok(PacketId::DetachService),
            0x3 => Ok(PacketId::LuaServices),
            _ => Err("Command can only include known values to the Command enum"),
        }
    }
}
