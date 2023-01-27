use std::{path::PathBuf, process::Stdio, collections::HashMap};

use log::{debug, info};
use tokio::{process::{Child, Command}, io::AsyncReadExt};

pub struct Service {
    pub id: String,
    pub name: String,
    pub cmd: String,
    pub cmd_args: Vec<String>,
    pub path: PathBuf, 
    pub child_process: Child,
}

pub struct ServiceAttacher {
    pub services: HashMap<String, Service>,
}

impl ServiceAttacher {
    pub fn attach(&mut self, name: String, cmd: String, cmd_args: Vec<String>, path: PathBuf) {

            let mut child = Command::new(cmd.clone())
                .args(&cmd_args[..])
                .current_dir(path.clone())
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to spawn NodeJS process");

            let mut stdout = child.stdout.take().unwrap();
            
            let service = Service {
                id: "a1b2c".into(),
                cmd: cmd.clone(),
                name: name.clone(),
                cmd_args: cmd_args.clone(),
                path: path.clone(),
                child_process: child,
            };

            self.services.insert(name, service);
            
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
}
