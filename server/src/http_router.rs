use std::future;

use crate::service_attacher::Attachable;
use crate::{prelude::*, service_attacher::HttpAttachable};
use actix_web::dev::Server;
use actix_web::web::Data;
use actix_web::{
    http::header::Header,
    web::{self, BytesMut},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use log::debug;
use reqwest::{header, Client};

async fn forward_request(
    client: web::Data<Client>,
    req: HttpRequest,
    base_url: web::Data<String>,
    body: web::Bytes,
) -> impl Responder {
    let target_url = format!("{}", base_url.as_str());
    debug!("Forward the request to {}", target_url);
    let actix_headers = req.headers().clone();
    let mut reqwest_headers = header::HeaderMap::new();

    actix_headers.iter().for_each(|value| {
        let (header_name, header_value) = value.clone();
        reqwest_headers.insert(header_name.clone(), header_value.clone());
    });

    let res = client
        .request(req.method().clone(), &target_url)
        .headers(reqwest_headers)
        .body(body.to_vec())
        .send()
        .await
        .unwrap();

    HttpResponse::build(res.status().into()).body(res.text().await.unwrap())
}
pub fn http_router(http_services: Vec<HttpAttachable>) -> Server {
    debug!("attaching http to port 9000");

    HttpServer::new(move || {
        let http_client = Client::new();
        let mut app = App::new().app_data(Data::new(http_client.clone()));

        for service in http_services.clone() {
            let host = f!("http://localhost:{}", service.port);
            let route = f!("/{}", service.name);

            debug!("Routing the requests at {} to {}", route, host);
            app = app.app_data(Data::new(host.clone()));
            app = app.route(route.as_str(), web::post().to(forward_request));
        }

        return app;
    })
    .bind("127.0.0.1:9000")
    .unwrap()
    .run()
}
async fn manual_hello() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}
