use std::{env, process};
use slack::{self, RtmClient};

mod queue;

/// Given the body of a post to Slack, determine someone mentioned the Queue app
fn is_app_mention(text: &str) -> bool {
	text.contains(queue::QUEUE_UID)
}

impl slack::EventHandler for queue::Queue {
	fn on_event(&mut self, cli: &RtmClient, event: slack::Event) {
		match event {
			slack::Event::Message(message) => {
				if let slack::Message::Standard(ms) = *message {
					// The content of the message
					let text = ms.text.unwrap_or_default();
					if is_app_mention(&*text) {
						// Who posted the message
						let user = ms.user.expect("User does not exist");
						// What to send back to Slack
						let response = self.determine_response(
							queue::User(String::from(user)),
							text.as_str()
						);
						// The channel the message was posted in
						let chan = ms.channel.expect("Channel does not exist");
						// Send 'em back!
						let _ = cli.sender().send_message(&*chan, &*response);
					}
				}
			},
			_ => (),
		};
	}

	fn on_close(&mut self, _cli: &RtmClient) {
		println!("on_close");
	}

	fn on_connect(&mut self, cli: &RtmClient) {
		println!("on_connect");
		let botspam_chan_id = cli
			.start_response()
			.channels
			.as_ref()
			.and_then(|chans| {
				chans
					.iter()
					.find(|chan| match chan.name {
						None => false,
						Some(ref name) => name == "botspam",
					})
			})
			.and_then(|chan| chan.id.as_ref())
			.expect("channel botspam not found")
		;
		let _ = cli.sender().send_message(&botspam_chan_id, "I\'m baaack!");
	}
}

fn main() -> Result<(), slack::error::Error> {
	let args = env::args().collect::<Vec<String>>();
	let api_key = match args.len() {
		0 | 1 => {
			eprintln!("No API key in args! Usage: cargo run --bin slack_main --features slack -- <api-key>");
			process::exit(-1);
		},
		nargs => args[nargs - 1].clone(),
	};
	let mut queue = queue::Queue::new();
	RtmClient::login_and_run(&api_key, &mut queue)
}