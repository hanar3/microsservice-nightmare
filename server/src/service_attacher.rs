use crate::prelude::*;
use std::{collections::HashMap, path::PathBuf, process::Stdio};

use log::{debug, info};
use packet::Service;
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
};
use uuid::Uuid;

pub struct Attachable {
    pub id: String,
    pub name: String,
    pub cmd: String,
    pub cmd_args: Vec<String>,
    pub path: PathBuf,
    pub child_process: Option<Child>,
}

impl TryFrom<Service> for Attachable {
    type Error = Error;

    fn try_from(service: Service) -> Result<Self> {
        let svc_name = std::str::from_utf8(&service.svc_name)
            .map_err(|e| Error::Generic(e.to_string()))
            .unwrap();
        let svc_path = std::str::from_utf8(&service.svc_path)
            .map_err(|e| Error::Generic(e.to_string()))
            .unwrap();
        let shell_cmd = std::str::from_utf8(&service.cmd)
            .map_err(|e| Error::Generic(e.to_string()))
            .unwrap();
        let cmd_args: Vec<String> = std::str::from_utf8(&service.cmd_args)
            .map_err(|e| Error::Generic(e.to_string()))
            .unwrap()
            .split(",")
            .collect::<Vec<&str>>()
            .iter()
            .map(|arg| arg.to_string())
            .collect();

        Ok(Attachable {
            id: Uuid::new_v4().to_string(),
            name: svc_name.to_string(),
            path: PathBuf::from(svc_path),
            cmd: shell_cmd.to_string(),
            cmd_args,
            child_process: None,
        })
    }
}

pub struct ServiceAttacher {
    pub services: HashMap<String, Attachable>,
}

impl ServiceAttacher {
    pub(crate) fn attach(&mut self, mut attachable: Attachable) {
        let mut child = Command::new(attachable.cmd.clone())
            .args(&attachable.cmd_args[..])
            .current_dir(attachable.path.clone())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn process");

        let mut stdout = child.stdout.take().unwrap();

        attachable.child_process = Some(child);

        self.services
            .insert(f!("{}:{}", attachable.name, attachable.id), attachable);

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
