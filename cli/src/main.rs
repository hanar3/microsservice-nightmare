use deku::prelude::*;
use std::error::Error;
use tokio::{io::AsyncReadExt, net::TcpStream};

#[derive(Debug, DekuRead, DekuWrite)]
struct PacketHead {
    id: u8,
    data_size: u8,
}
#[derive(Debug, DekuRead, DekuWrite)]
struct Message {
    id: u8,

    data_size: u8,
    #[deku(count = "data_size")]
    data: Vec<u8>,
}

#[derive(Debug, DekuRead, DekuWrite)]
struct AttachService {
    name_len: u8,
    #[deku(count = "name_len")]
    svc_name: Vec<u8>,

    svc_type: u8,

    svc_path_len: u8,
    #[deku(count = "svc_path_len")]
    svc_path: Vec<u8>,

    cmd_len: u8,
    #[deku(count = "cmd_len")]
    cmd: Vec<u8>,

    cmd_args_len: u8,
    #[deku(count = "cmd_args_len")]
    cmd_args: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (mut reader, mut writer) = stream.split();
    let svc_name = b"bank_server";
    let cmd = b"npm";
    let args = b"run,debug";
    let path = b"/Users/hanar3/Documents/bitbucket/Etana/bank_server";

    let attach_svc = AttachService {
        cmd_len: cmd.len() as u8,
        cmd: cmd.to_vec(),

        cmd_args: args.to_vec(),
        cmd_args_len: args.len() as u8,

        name_len: svc_name.len() as u8,
        svc_name: svc_name.to_vec(),

        svc_type: 1,

        svc_path: path.to_vec(),
        svc_path_len: path.len() as u8,
    };

    let attach_svc_bytes = AttachService::to_bytes(&attach_svc).unwrap();

    let attach_service_msg = Message {
        id: 0x01,
        data_size: attach_svc_bytes.len() as u8,
        data: attach_svc_bytes.to_vec(),
    };

    let msg_bytes = Message::to_bytes(&attach_service_msg).unwrap();
    println!(
        "Attach service size {}",
        std::mem::size_of::<AttachService>()
    );
    println!("Sending bytes: {:?}", msg_bytes);
    // Send a message to the server
    writer.try_write(&msg_bytes[..]).unwrap();

    // Read the response from the server
    let mut buffer = [0; 1024];
    let n = reader.read(&mut buffer).await?;
    let response = std::str::from_utf8(&buffer[..n]).unwrap();
    println!("Response from server: {}", response);

    Ok(())
}
