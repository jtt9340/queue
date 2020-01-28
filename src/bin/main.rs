//! The main logic of the slack bot. This will eventually be split into separate files.

use std::{env, io, net::Ipv4Addr, process, sync::Arc};

use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use serde::Serialize;
use serde_json as json;

pub use print_queue::queue;
pub use print_queue::user;

/// The IP Address we are connecting to
// const IP_ADDR: [u8; 4] = [213u8, 108, 105, 162];
// Temporarily using localhost
const IP_ADDR: Ipv4Addr = Ipv4Addr::LOCALHOST;
/// Which port number the host is bound to
const PORT: u16 = 3152;
/// The URL to make a POST request to in order to post in the chat
const CHAT_POSTMESSAGE: &str = "https://slack.com/api/chat.postMessage";

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
	// TODO: AppState is implicitly wrapped in an Arc which only provides an immutable reference to
	//   the underlying data, so we need to figure out how to mutate *just* the queue::Queue
	queue: queue::Queue,
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
				queue: queue::Queue::new(),
			})
			.route("/slack/events", web::post().to(post_handler))
	})
		.bind(socket_addr)?
		.run()
		.await
}

/// Handle a POST request sent from the Slack API. This POST request contains a JSON payload
/// containing all sorts of information about a particular event, including
/// * What event was it?
/// * What user caused the event to happen?
/// * In what channel did this event occur?
/// * And more! See https://api.slack.com/events-api#receiving_events
///
/// The `json_payload` parameter corresponds to this, and the `app_state` parameter contains the
/// data necessary to send a POST request to Slack.
///
/// According to the Slack documentation, this function must send back a 200 OK HTTP status within 5
/// seconds of being invoked, so the heavy work of building up a POST request to send back to Slack
/// so that the Slack bot can make a post in the chat is done in a future created by the `send_request`
/// function.
async fn post_handler((web::Json(json_payload), app_state): (web::Json<json::Value>, web::Data<AppState>)) -> impl Responder {
	println!("{:#?}", json_payload);
	actix::spawn(send_request(json_payload, app_state.into_inner()));
	HttpResponse::Ok()
}

/// Send a POST request to Slack based on what a user just typed in. The `payload` and `app_state`
/// parameters are the same as described in the documentation for `post_handler`.
async fn send_request(payload: json::Value, app_state: Arc<AppState>) {
	// Did somebody @ the app?
	let is_app_mention = payload
		.pointer("/event/type")
		.and_then(json::Value::as_str)
		.map_or(false, |e| e == "app_mention")
		;

	if is_app_mention {
		// The content of the message the user posted
		let text = payload
			.pointer("/event/text")
			.and_then(json::Value::as_str)
			.unwrap_or_default()
			;

		// The Slack user who posted a message
		let user = payload
			.pointer("/event/user")
			.and_then(json::Value::as_str)
			.unwrap()
			;

		let response = determine_response(
			user::User(String::from(user)),
			text,
			&mut app_state.queue
		);

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
				text: response,
				channel: chan.to_string(),
			})
			.send()
			.expect("Failed sending a POST request to chat.postMessage")
			;
	}
}

/// Given the text of what someone posted when "at-ing" Queue, determine how to modify the queue and
/// what to say back.
///
/// Parameter `user` is the user (a string of the format UXXXXXXXX) who said `body`. `queue` is the
/// `Queue` of `User`s that mutated after calling this function
/// Currently, this function has the side-effect of mutating the state of the queue passed in
fn determine_response(user: user::User, body: &str, queue: &mut queue::Queue) -> String {
	match body.to_lowercase().as_str() {
		"add" => {
			if queue.add_user(user) {
				format!("Okay <@{}>, I have added you to the queue", user.0)
			} else {
				String::from("You are already in the queue")
			}
		},
		"cancel" => {
			if queue.remove_user(user) {
				format!("Okay <@{}>, I have removed you from the queue", user.0)
			} else {
				String::from("You weren't in the queue to begin with")
			}
		},
		"done" => {
			if queue
				.peek_first_user_in_line()
				.map_or(false, |u| *u == user)
			{
				/* It *should* be safe to unwrap() here because the condition ensures there is a
				first user in line in the first place */
				let user = queue.remove_first_user_in_line().unwrap();
				format!("Okay <@{}>, you have been removed from the front of the queue", user.0)
			} else {
				String::from("You cannot be done; you are not at the front of the line")
			}
		},
		"show" => format!("{:?}", queue),
		_ => String::from("unrecognized command. Your options are: add, cancel, done, and show")
	}
}