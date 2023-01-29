use crate::prelude::*;
use env_logger::{self, Env};
use lazy_static::lazy_static;
use log::info;
use packet::{Message, PacketHeader};
use std::{collections::HashMap, io::Read, io::Write, net::TcpListener, sync::RwLock};
mod error;
mod prelude;

mod http_router;
mod message_parser;
mod service_attacher;

#[allow(unreachable_code)]
fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    env_logger::init_from_env(Env::default().default_filter_or("debug"));
    let mut buffer = [0; 1024];
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        info!("Connection established");
        
        let data = stream.read(&mut buffer).unwrap();
        let packet_header = PacketHeader::try_from(&buffer[0..2]).unwrap();
        let message_size = std::mem::size_of::<PacketHeader>() + packet_header.data_size as usize;
        
        // Extract only the valid parts of the buffer (exclude trailing 0's)
        let valid_buffer = &buffer[0..message_size];
        message_parser::parse_message(Message::try_from(valid_buffer).unwrap());
        info!("Received data from stream: {:?}", buffer);
        stream.write(b"ok!").unwrap();
    }

    Ok(())
}

lazy_static! {
    static ref SERVICE_ATTACHER: RwLock<service_attacher::ServiceAttacher> =
        RwLock::new(service_attacher::ServiceAttacher {
            services: HashMap::new(),
            http_server_handle: None,
        });
}
