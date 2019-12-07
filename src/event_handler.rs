//! The main logic of the slack bot. This will eventually be split into separate files.

use actix_web::{HttpServer, App, web, Responder, HttpResponse};
use serde::Serialize;
use serde_json as json;

use std::net::Ipv4Addr;

/// The IP Address we are connecting to
// const IP_ADDR: [u8; 4] = [213u8, 108, 105, 162];
const IP_ADDR: Ipv4Addr = Ipv4Addr::LOCALHOST; // Temporarily using localhost
/// Which port number the host is bound to
const PORT: u16 = 3152;
/// The token that is sent to the Slack API when the bot wants to post something
const BOT_TOKEN: &str = "***REMOVED***";

// Insert struct definition here that represents what slack sends when it sends
// a POST request

/// A type encapsulating the POST request that the Slack bot will send to the Slack API when it wants
/// to respond.
#[derive(Debug, Serialize)]
struct Response {
	text: String,
	channel: String,
}

fn main() {
	// Which public url to connect to
	// In the future, it should be alchemi.dev:3152 (or something)
	// let socket_addr = SocketAddr::from((IP_ADDR, PORT));
	// but right now it is localhost:3152
	let socket_addr = (IP_ADDR, PORT);
	HttpServer::new(|| {
		App::new()
			.route("/slack/events", web::post().to(post_handler))
	})
		.bind(socket_addr)
		.expect("Cannot bind to port 8000")
		.run()
		.expect("Failed to run event handler web server")
	;
}

fn post_handler(web::Json(payload): web::Json<json::Value>) -> impl Responder {
	println!("{:#?}", payload);
	// actix::spawn(send_request());
	HttpResponse::Ok()
}

/*
async fn send_request() -> Result<(), ()> {
	// Use reqwest to make a POST request
	Ok(())
}*/
