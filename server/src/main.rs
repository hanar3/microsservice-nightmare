use std::io::{Cursor};
use std::mem::size_of;
use std::process::Stdio;

use log::{info, debug};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter, Stdout, AsyncBufReadExt};
use tokio::net::{TcpStream, TcpListener};
use deku::prelude::*;
use env_logger::{self, Env};
use tokio::process::{Child, Command};


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
    #[deku( count = "name_len")]
    svc_name: Vec<u8>,

    svc_type: u8,
    
    svc_path_len: u8,
    #[deku(count = "svc_path_len")]
    svc_path: Vec<u8>,

    cmd_len: u8,
    #[deku(count = "cmd_len")]
    cmd: Vec<u8> 
}


enum AppCommand {
    AttachService = 0x1,
    DetachService = 0x2,
}

impl TryFrom<u8> for AppCommand {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x1 => Ok(AppCommand::AttachService),
            0x2 => Ok(AppCommand::DetachService),
            _ => Err("Command can only include known values to the Command enum")
        }    
    }
}

impl Message {
    async fn parse_command(&self) {

       let cmd = AppCommand::try_from(self.id).unwrap();
       // [1, 4, 84, 69, 83, 84, 1, 1, 15, 4, 101, 99, 104, 111]

       match cmd {
           AppCommand::AttachService => {
            debug!("AttachService command received, parsing service from data... {:?}", &self.data);
            let command_data = AttachService::try_from(&self.data[..]);
            debug!("AttachService result: {:?}", command_data);

            let attach_service = command_data.unwrap();
              
            let svc_name = std::str::from_utf8(&attach_service.svc_name).unwrap();
            let svc_path = std::str::from_utf8(&attach_service.svc_path).unwrap();
            let shell_cmd = std::str::from_utf8(&attach_service.cmd).unwrap();

            let mut child = Command::new("npm")
                .args(&["run", "debug"])
                .current_dir("/Users/hanar3/Documents/bitbucket/Etana/bank_server")
                .stderr(Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()
                .expect("Failed to spawn NodeJS process");


            let mut stdout = child.stdout.take().unwrap();
            tokio::spawn(async move {
                let mut stdout_buf = [0; 4096];
                loop {
                    match stdout.read(&mut stdout_buf).await {
                        Ok(n) if n == 0 => break,
                        Ok(n) => {
                            let output = std::str::from_utf8(&stdout_buf[..n]).unwrap();
                            println!("{}", output);
                        }
                        Err(e) => {
                            println!("Error reading stdout: {}", e);
                            break;
                        }
                    }
                }
            });
           debug!("Attaching service: {}, at: {}, with command: {}", svc_name, svc_path, shell_cmd);
           },
           AppCommand::DetachService => {} } 
    }
}


#[allow(unreachable_code)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = TcpListener::bind("127.0.0.1:8080").await?;
    env_logger::init_from_env(Env::default().default_filter_or("server"));
    let attach_service_example =  AttachService { name_len: 4, svc_name: vec![0x54, 0x45, 0x53, 0x54], svc_type: 0x1, svc_path_len: 0, svc_path: vec![], cmd_len: 4, cmd: vec![0x65,0x63,0x68,0x6F]  };
    let message = Message { id: 1, data_size: 12, data: AttachService::to_bytes(&attach_service_example).unwrap() };
    println!("Messagw bytes: {:x?}", Message::to_bytes(&message));

    loop {
        let (socket, host) = listener.accept().await?;
        tokio::spawn(async move {
            info!("Connection received from IP {}", host.ip());
            let mut reader = BufReader::new(socket);
            let mut buffer = [0; 1024];

            while let Ok(n) = reader.read(&mut buffer[..]).await {
                if n == 0 {
                    info!("Closed connection with IP {}", host.ip());
                    break;
                }

                let packet_head = PacketHead::try_from(&buffer[..2]).unwrap();
                if packet_head.id <= 0 { panic!("Message ID cannot be 0"); }
                
                let msg_bytes = std::mem::size_of::<PacketHead>() + (packet_head.data_size as usize);                

                let message = Message::try_from(&buffer[0..msg_bytes]).unwrap(); 
    
                message.parse_command().await; 
            }
        });
    }

    Ok(())
}
