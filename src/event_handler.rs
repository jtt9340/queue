//! The main logic of the slack bot. This will eventually be split into separate files.

use std::{env, io, net::Ipv4Addr, process, sync::Arc};

use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use serde::Serialize;
use serde_json as json;

/// The IP Address we are connecting to
// const IP_ADDR: [u8; 4] = [213u8, 108, 105, 162];
const IP_ADDR: Ipv4Addr = Ipv4Addr::LOCALHOST;
// Temporarily using localhost
/// Which port number the host is bound to
const PORT: u16 = 3152;

// Insert struct definition here that represents what slack sends when it sends
// a POST request

/// A type encapsulating the POST request that the Slack bot will send to the Slack API when it wants
/// to respond.
#[derive(Debug, Serialize)]
struct Response {
	text: String,
	channel: String,
}

/// The data that all HTTP request handlers need access to
#[derive(Debug)]
struct AppState {
	bot_token: String,
	session: reqwest::Client,
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
	/* Since the Slack bot API token is sensitive data, we won't store it as a constant that will be
	committed to git. While we could store that constant in a file that is .gitignore-ed, it is easier
	to get that token from the commandline. */
	let bot_token = match env::args().nth(1) {
		Some(token) => token,
		None => {
			eprintln!("Supply a BOT_TOKEN at the command line");
			process::exit(-1);
		}
	};

	println!("{:?}", bot_token);

	// Which public url to connect to
	// In the future, it should be alchemi.dev:3152 (or something)
	// let socket_addr = SocketAddr::from((IP_ADDR, PORT));
	// but right now it is localhost:3152
	// TODO: Upgrade this to actix-web 2.0.0?
	let socket_addr = (IP_ADDR, PORT);
	HttpServer::new(move || {
		App::new()
			.data(AppState {
				bot_token: bot_token.clone(),
				session: reqwest::Client::new(),
			})
			.route("/slack/events", web::post().to(post_handler))
	})
		.bind(socket_addr)?
		.run()
		.await
}

async fn post_handler((web::Json(json_payload), app_state): (web::Json<json::Value>, web::Data<AppState>)) -> impl Responder {
	println!("{:#?}", json_payload);
	actix::spawn(send_request(json_payload, app_state.into_inner()));
	HttpResponse::Ok()
}

async fn send_request(payload: json::Value, app_state: Arc<AppState>) {
	// Did somebody @ the app?
	let is_app_mention = payload
		.pointer("/event/type")
		.map(json::Value::as_str)
		.flatten()
		.map(|e| e == "app_mention")
		.unwrap_or(false)
		;

	if is_app_mention {
		// Did the user ask for the Slack bot to tell him or her a joke?
		let wants_a_joke = payload
			.pointer("/event/text")
			.map(json::Value::as_str)
			.flatten()
			.map(|t| t.contains("tell me a joke"))
			.unwrap_or(false)
			;

		if wants_a_joke {
			/*
				Use reqwest to make a POST request to chat.postMessage using bot's token
				Text: Hello {user}! Knock, knock.
				Channel: {channel}

				where user = /event/user
				and channel = /event/channel
			*/
			let user = payload
				.pointer("/event/user")
				.map(json::Value::as_str)
				.flatten()
				.unwrap()
				;
			let chan = payload
				.pointer("/event/channel")
				.map(json::Value::as_str)
				.flatten()
				.unwrap()
				;

			let _ = app_state
				.session
				.post("https://slack.com/api/chat.postMessage")
				.bearer_auth(app_state.bot_token.clone())
				.json(&Response {
					text: format!("Hello <@{}>! Knock, knock.", user),
					channel: chan.to_string(),
				})
				.send()
				.expect("Failed sending a POST request to chat.postMessage")
				;
		}
	}
}