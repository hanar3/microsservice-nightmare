use crate::prelude::*;
use std::{
    collections::HashMap,
    io::Read,
    path::PathBuf,
    process::Stdio,
    process::{Child, Command},
    sync::mpsc,
    thread::{self, JoinHandle},
};

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
    pub attachable_type: u8,
    pub child_process: Option<Child>,
    pub thread_handle: Option<JoinHandle<()>>,
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
            attachable_type: service.svc_type,
            thread_handle: None,
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

        // Save child handle
        let mut stdout = child.stdout.take().unwrap();
        attachable.child_process = Some(child);

        let thread_handle = thread::spawn(move || {
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
        attachable.thread_handle = Some(thread_handle);
        // Save attachable
        self.services.insert(attachable.name.clone(), attachable);
        self.attach_http_services();
    }

    fn attach_http_services(&mut self) {
        // If we already have a http server open, let's shut it down
        if let Some(handle) = &self.http_server_handle {
            info!("Gracefully shutting down http server");
            rt::System::new().block_on(handle.stop(true));
        };

        let http_service_map: HashMap<String, HttpAttachable> =
            self.services.iter().fold(HashMap::new(), |mut acc, value| {
                let (_, service) = value;
                if service.attachable_type == 1 {
                    acc.insert(
                        service.name.clone(),
                        HttpAttachable::try_from(service).unwrap(),
                    );
                }
                return acc;
            });

        let (tx, rx) = mpsc::channel();
        log::debug!("spawning thread for server");
        thread::spawn(move || {
            let server_future = run_http_server(tx, http_service_map.clone());
            rt::System::new().block_on(server_future)
        });

        self.http_server_handle = Some(rx.recv().unwrap());
    }
}

// Should be moved to separate file after
// Run actix in a thread
async fn forward_request(
    client: web::Data<Client>,
    req: HttpRequest,
    http_services: web::Data<HashMap<String, HttpAttachable>>,
    body: web::Bytes,
) -> impl Responder {
    info!("Received a new request, attempting to find a service to route it to");
    let path_no_leading = req.path().chars().skip(1).collect::<String>(); // Remove the leading / from
                                                                          // the path
    let path_parts: Vec<&str> = path_no_leading.split("/").collect();

    debug!("path_stem: {}, path_parts: {:?}", path_parts[0], path_parts);
    let path_to_forward = if path_parts.len() > 1 {
        f!("{}", path_parts[1..].join("/"))
    } else {
        "".to_string()
    };
    debug!("path_to_forward: {}", path_to_forward);

    let service_to_forward = http_services.get(path_parts[0]).unwrap();

    // Construct request URL
    let request_url = f!(
        "http://localhost:{}/{}",
        service_to_forward.port,
        path_to_forward
    );

    let actix_headers = req.headers().clone();
    let mut reqwest_headers = header::HeaderMap::new();

    actix_headers.iter().for_each(|value| {
        let (header_name, header_value) = value.clone();
        reqwest_headers.insert(header_name.clone(), header_value.clone());
    });

    let res = client
        .request(req.method().clone(), request_url)
        .headers(reqwest_headers)
        .body(body.to_vec())
        .send()
        .await
        .unwrap();

    HttpResponse::build(res.status().into()).body(res.text().await.unwrap())
}

async fn run_http_server(
    tx: mpsc::Sender<ServerHandle>,
    http_services: HashMap<String, HttpAttachable>,
) -> Result<()> {
    info!("starting HTTP server at localhost:9000");

    let server = HttpServer::new(move || {
        let http_client = Client::new();
        let app = App::new()
            .app_data(Data::new(http_client.clone()))
            .app_data(Data::new(http_services.clone()))
            .route("/{tail}*", web::patch().to(forward_request))
            .route("/{tail}*", web::put().to(forward_request))
            .route("/{tail}*", web::delete().to(forward_request))
            .route("/{tail}*", web::get().to(forward_request))
            .route("/{tail}*", web::post().to(forward_request));
        return app;
    })
    .bind("127.0.0.1:9000")
    .unwrap()
    .workers(2)
    .run();

    let _ = tx.send(server.handle());
    server.await.map_err(|e| Error::Generic(e.to_string()))
}
