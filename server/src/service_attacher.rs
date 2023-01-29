use crate::prelude::*;
use std::{collections::HashMap, path::PathBuf, process::Stdio, sync::mpsc, thread, process::{Child, Command}, io::Read};

use actix_web::{
    dev::{Server, ServerHandle},
    middleware, rt,
    web::{self, Data},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use log::{debug, info};
use packet::Service;
use reqwest::{header, Client};

use uuid::Uuid;

#[derive(Debug)]
pub struct Attachable {
    pub id: String,
    pub name: String,
    pub cmd: String,
    pub cmd_args: Vec<String>,
    pub path: PathBuf,
    pub child_process: Option<Child>,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct HttpAttachable {
    pub id: String,
    pub name: String,
    pub route_path: String,
    pub port: u16,
}

impl TryFrom<&Attachable> for HttpAttachable {
    type Error = Error;
    fn try_from(value: &Attachable) -> Result<HttpAttachable> {
        Ok({
            HttpAttachable {
                id: value.id.clone(),
                name: value.name.clone(),
                port: value.port,
                route_path: value.name.clone(),
            }
        })
    }
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
            port: service.svc_port,
        })
    }
}

pub struct ServiceAttacher {
    pub services: HashMap<String, Attachable>,
    pub http_server_handle: Option<ServerHandle>,
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

        if let Some(handle) = &self.http_server_handle {
            info!("Gracefully shutting down http server");
            rt::System::new().block_on(handle.stop(true));
        }

        let http_services: Vec<HttpAttachable> = self
            .services
            .iter()
            .map(|(_, attachable)| HttpAttachable::try_from(attachable).unwrap())
            .collect();

        let (tx, rx) = mpsc::channel();
        log::info!("spawning thread for server");
        thread::spawn(move || {
            let server_future = run_http_server(tx, http_services.clone());
            rt::System::new().block_on(server_future)
        });

        self.http_server_handle = Some(rx.recv().unwrap());
        thread::spawn(move || {
            let mut stdout_buf = [0; 4096];
            loop {
                match stdout.read(&mut stdout_buf) {
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

// Should be moved to separate file after
// Run actix in a thread
async fn forward_request(
    client: web::Data<Client>,
    req: HttpRequest,
    path_to_url: web::Data<HashMap<String, String>>,
    body: web::Bytes,
) -> impl Responder {
    let path = req.path();
    debug!("Received request at path: {}", path);
    let forward_url = path_to_url.get(path).unwrap();
    
    let actix_headers = req.headers().clone();
    let mut reqwest_headers = header::HeaderMap::new();

    actix_headers.iter().for_each(|value| {
        let (header_name, header_value) = value.clone();
        reqwest_headers.insert(header_name.clone(), header_value.clone());
    });

    let res = client
        .request(req.method().clone(), forward_url)
        .headers(reqwest_headers)
        .body(body.to_vec())
        .send()
        .await
        .unwrap();

    HttpResponse::build(res.status().into()).body(res.text().await.unwrap())
}

async fn run_http_server(
    tx: mpsc::Sender<ServerHandle>,
    http_services: Vec<HttpAttachable>,
) -> Result<()> {
    info!("starting HTTP server at localhost:9000");
    let mut route_map: HashMap<String, String> = HashMap::new();

    http_services.iter().for_each(|service| {
        route_map.insert(f!("/{}", service.name), f!("http://localhost:{}", service.port));
    });


    let server = HttpServer::new(move || {
        let http_client = Client::new();
        let mut app = App::new().app_data(Data::new(http_client.clone()));
        app = app.app_data(Data::new(route_map.clone()));
        for service in http_services.clone() {
            let route = f!("/{}", service.name);
            app = app.route(route.as_str(), web::post().to(forward_request));
        }

        return app;
    })
    .bind("127.0.0.1:9000")
    .unwrap()
    .workers(2)
    .run();

    let _ = tx.send(server.handle());
    server.await.map_err(|e| Error::Generic(e.to_string()))
}
