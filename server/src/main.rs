use std::process::Stdio;
use env_logger::{self, Env};
use log::{debug, info};
use packet::{AppCommand, AttachService, Message, PacketHead};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::Command;

async fn parse_command(msg: Message) {
    let cmd = AppCommand::try_from(msg.id).unwrap();
    // [1, 4, 84, 69, 83, 84, 1, 1, 15, 4, 101, 99, 104, 111]

    match cmd {
        AppCommand::AttachService => {
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
            let cmd_args: Vec<&str> = std::str::from_utf8(&attach_service.cmd_args)
                .unwrap()
                .split(",")
                .collect();

            let mut child = Command::new(shell_cmd)
                .args(&cmd_args[..])
                .current_dir(svc_path)
                .stderr(Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()
                .expect("Failed to spawn NodeJS process");

            let mut stdout = child.stdout.take().unwrap();

            debug!(
                "Attaching service: {}, at: {}, with command: {} and args: {:?}",
                svc_name.clone(),
                svc_path,
                shell_cmd,
                cmd_args
            );

            tokio::spawn(async move {
                let mut stdout_buf = [0; 4096];
                loop {
                    match stdout.read(&mut stdout_buf).await {
                        Ok(n) if n == 0 => break,
                        Ok(n) => {
                            let output = std::str::from_utf8(&stdout_buf[..n]).unwrap();
                            info!("service: {}", output);
                        }
                        Err(e) => {
                            println!("Error reading stdout: {}", e);
                            break;
                        }
                    }
                }
            });
        }
        AppCommand::DetachService => {}
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

                let packet_head = PacketHead::try_from(&buffer[..2]).unwrap();
                if packet_head.id <= 0 {
                    panic!("Message ID cannot be 0");
                }

                let msg_bytes =
                    std::mem::size_of::<PacketHead>() + (packet_head.data_size as usize);
                info!("packet_head {:?}", packet_head);
                let message = Message::try_from(&buffer[0..msg_bytes]).unwrap();

                parse_command(message).await;
                writer.write(b"ok").await.unwrap();
            }
        });
    }

    Ok(())
}
