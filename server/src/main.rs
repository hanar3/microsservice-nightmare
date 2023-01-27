use crate::prelude::*;
use env_logger::{self, Env};
use lazy_static::lazy_static;
use log::info;
use packet::{Message, PacketHeader};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

mod error;
mod prelude;

mod message_parser;
mod service_attacher;
#[allow(unreachable_code)]
#[tokio::main]
async fn main() -> Result<()> {
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
                    info!("Gracefully closed connection with IP {}", host.ip());
                    break;
                }

                let packet_head = PacketHeader::try_from(&buffer[..2]).unwrap();
                let msg_bytes =
                    std::mem::size_of::<PacketHeader>() + (packet_head.data_size as usize);
                let message = Message::try_from(&buffer[0..msg_bytes]).unwrap();
                message_parser::parse_message(message).await;
                writer.write(b"ok").await.unwrap();
            }
        });
    }

    Ok(())
}

lazy_static! {
    static ref SERVICE_ATTACHER: RwLock<service_attacher::ServiceAttacher> =
        RwLock::new(service_attacher::ServiceAttacher {
            services: HashMap::new()
        });
}
