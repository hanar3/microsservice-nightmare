use std::collections::HashMap;
use std::path::PathBuf;
use env_logger::{self, Env};
use log::{debug, info};
use packet::{PacketId, AttachService, Message, PacketHeader};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use lazy_static::lazy_static;
use tokio::sync::RwLock;
mod service_attacher;

async fn parse_command(msg: Message) {
    let cmd = PacketId::try_from(msg.id).unwrap();
    // [1, 4, 84, 69, 83, 84, 1, 1, 15, 4, 101, 99, 104, 111]

    match cmd {
        PacketId::AttachService => {
            debug!(
                "AttachService command received, parsing service from data... {:?}",
                &msg.data
            );
            let command_data = AttachService::try_from(&msg.data[..]);
            debug!("AttachService result: {:?}", command_data);

            let attach_service = command_data.unwrap();
            let svc_name = std::str::from_utf8(&attach_service.svc_name).unwrap();
            let svc_path = std::str::from_utf8(&attach_service.svc_path).unwrap();
            let shell_cmd = std::str::from_utf8(&attach_service.cmd).unwrap();
            let cmd_args: Vec<String> = std::str::from_utf8(&attach_service.cmd_args)
                .unwrap()
                .split(",")
                .collect::<Vec<&str>>()
                .iter()
                .map(|arg| arg.to_string())
                .collect();
            let mut service_attacher = SERVICE_ATTACHER.write().await;
           service_attacher.attach(svc_name.into(), shell_cmd.into(), cmd_args, PathBuf::try_from(svc_path).unwrap()); 
        }
        PacketId::DetachService => {}
    }
}

#[allow(unreachable_code)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    env_logger::init_from_env(Env::default().default_filter_or("server"));

    loop {
        let (mut socket, host) = listener.accept().await?;
        tokio::spawn(async move {
            info!("Connection received from IP {}", host.ip());
            let (reader, mut writer) = socket.split();
            let mut buf_reader = BufReader::new(reader);
            let mut buffer = [0; 1024];

            while let Ok(n) = buf_reader.read(&mut buffer[..]).await {
                if n == 0 {
                    info!("Closed connection with IP {}", host.ip());
                    break;
                }

                let packet_head = PacketHeader::try_from(&buffer[..2]).unwrap();


                // The total size of the message will always be Header + data_size (the second byte
                // of every message)
                
                let msg_bytes = std::mem::size_of::<PacketHeader>() + (packet_head.data_size as usize);
                info!("packet_head {:?}", packet_head);
                let message = Message::try_from(&buffer[0..msg_bytes]).unwrap();

                parse_command(message).await;
                writer.write(b"ok").await.unwrap();
            }
        });
    }

    Ok(())
}


lazy_static! {
    static ref SERVICE_ATTACHER: RwLock<service_attacher::ServiceAttacher> = RwLock::new(service_attacher::ServiceAttacher { services: HashMap::new() });
}
