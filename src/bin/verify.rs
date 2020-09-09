//! A "script" used to validate the slack events api. Since Queue is (not yet) running on a permanent
//! server, this needs to be run every time before working on Queue in order to validate the URL that
//! ngrok randomly generates each session.
//!
//! Run ngrok with: `ngrok http PORT` (see PORT below)

// use std::net::SocketAddr;
use std::{io, net::Ipv4Addr};

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;

/// The IP address we are connecting to
// const IP_ADDR: [u8; 4] = [213u8, 108, 105, 162];
/// Which port number the host is bound to
const PORT: u16 = 3152;

#[derive(Deserialize, Debug)]
struct Payload {
    token: String,
    challenge: String,
    r#type: String,
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    // Which public url to connect to
    // In the future, it should be alchemi.dev:3152 (or something else)
    // let socket_addr = SocketAddr::from((IP_ADDR, PORT));
    // but right now it is localhost:3152
    let socket_addr = (Ipv4Addr::LOCALHOST, PORT);
    // Let's set up a web server!
    HttpServer::new(|| App::new().route("/slack/events", web::post().to(post_handler)))
        .bind(socket_addr)?
        .run()
        .await
}

async fn post_handler(web::Json(response): web::Json<Payload>) -> impl Responder {
    println!("{:?}", response);
    HttpResponse::Ok().body(response.challenge)
}
