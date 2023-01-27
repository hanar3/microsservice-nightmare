use deku::prelude::*;

#[derive(Debug, DekuRead, DekuWrite)]
pub struct PacketHead {
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
pub struct AttachService {
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
}

pub enum AppCommand {
    AttachService = 0x1,
    DetachService = 0x2,
}

impl TryFrom<u8> for AppCommand {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x1 => Ok(AppCommand::AttachService),
            0x2 => Ok(AppCommand::DetachService),
            _ => Err("Command can only include known values to the Command enum"),
        }
    }
}
