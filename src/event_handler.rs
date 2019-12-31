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
/// The URL to make a POST request to in order to post in the chat
const CHAT_POSTMESSAGE: &str = "https://slack.com/api/chat.postMessage";

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

	println!("{}", bot_token);

	// Which public url to connect to
	// In the future, it should be alchemi.dev:3152 (or something)
	// let socket_addr = SocketAddr::from((IP_ADDR, PORT));
	// but right now it is localhost:3152
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
		.and_then(json::Value::as_str)
		.map_or(false, |e| e == "app_mention")
		;

	if is_app_mention {
		// Did the user ask for the Slack bot to tell him or her a joke?
		let wants_a_joke = payload
			.pointer("/event/text")
			.and_then(json::Value::as_str)
			.map_or(false, |t| t.contains("tell me a joke"))
			;

		if wants_a_joke {
			/*
				Use reqwest to make a POST request to chat.postMessage using bot's token
				Text: Hello {user}! Knock, knock.
				Channel: {channel}

				where user = /event/user
				and channel = /event/channel
			*/
			// The Slack user who posted a message
			let user = payload
				.pointer("/event/user")
				.and_then(json::Value::as_str)
				.unwrap()
				;

			// The channel that the message was posted in
			let chan = payload
				.pointer("/event/channel")
				.and_then(json::Value::as_str)
				.unwrap()
				;

			let _ = app_state
				.session
				.post(CHAT_POSTMESSAGE)
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
	// Continue the knock-knock joke
	let is_message = payload
		.pointer("/event/type")
		.and_then(json::Value::as_str)
		.map_or(false, |e| e == "message")
		;

	if is_message {
		let payload_event_text = payload
			.pointer("/event/text")
			.and_then(json::Value::as_str)
			;
		// What to respond with
		let response_text =
			if payload_event_text.map_or(false, |t| t.contains("Who’s there?")) {
				Some("Underwear")
			} else if payload_event_text.map_or(false, |t| t.contains("Underwear who?")) {
				Some("Ever underwear you're going?")
			} else {
				None
			};
		// Make call to chat.postMessage sending response_text using bot's token
		if let Some(response) = response_text {
			/* What channel to respond to
			   TODO: this is repeated code from above, make this entire function
				   more succinct and more DRY */
			let chan = payload
				.pointer("/event/channel")
				.and_then(json::Value::as_str)
				.unwrap()
				;

			let _ = app_state
				.session
				.post(CHAT_POSTMESSAGE)
				.bearer_auth(app_state.bot_token.clone())
				.json(&Response {
					text: response.to_string(),
					channel: chan.to_string(),
				})
				.send()
				.expect("Failed sending a POST request to chat.postMessage")
				;
		}
	}
}