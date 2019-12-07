//! An example that uses the [slack crate] to write a slack bot that tells a (bad) knock-knock joke.
//!
//! The idea is that a user initiates the knock-knock joke by "@-ing" the slack bot:
//! ```
//! Johnny Appleseed: @Queue tell me a joke
//! ```
//! The slack bot should respond with:
//! ```
//! Queue: Knock, knock?
//! ```
//! The user must reply with:
//! ```
//! Johnny Appleseed: Who's there?
//! ```
//! In order to get the response:
//! ```
//! Queue: Underwear
//! ```
//! The user's reponse to this should be:
//! ```
//! Johnny Appleseed: Underwear who?
//! ```
//! And the slack bot finally responds with:
//! ```
//! Queue: Ever underwear you're going?
//! ```
//! Any other response yields from the slack bot:
//! ```
//! I don't know how to respond.
//! ```
//! This get's annoying fast, especially since there is a bug in this file where the slack bot *only*
//! responds with the last line, i.e. `I don't know how to respond`. I originally used the slack crate
//! because the method of using [actix-web] to handle the POST requests that the slack API uses wasn't
//! working, but that was due to the fact that Queue wasn't installed into a channel. Now that the
//! actix-web POST request-handling method works, the attempt at using the slack crate has been abandoned.
//!
//! [slack crate]: https://crates.io/crates/slack
//! [actix-web]: https://actix.rs

use slack;
use slack::{Event, RtmClient, Message};

/*
Knock, knock.
Who's there?
Underwear
Underwear who?
Ever underwear you're going?
*/
struct MyHandler;

impl slack::EventHandler for MyHandler {
	fn on_event(&mut self, cli: &RtmClient, event: Event) {
		match event {
			// When the user first @'s Queue
			Event::DesktopNotification { ref content, ref channel, .. }
			if content.is_some() && content.as_ref().unwrap().contains("tell me a joke") => {
				let _ = cli.sender().send_message(channel.as_ref().unwrap(), "Knock, knock.");
			},
			// Responses after the initial "knock, knock"
			Event::Message(message) => {
				let (response_text, chan) = match *message {
					Message::Standard(ref ms) if ms.text.is_some() => {
						let text = ms.text.as_ref().unwrap();
						println!("{}", text);
						if text.contains("Who's there?") {
							("Underwear", ms.channel.as_ref())
						} else if text.contains("Underwear who?") {
							("Ever underwear you're going? 🤔", ms.channel.as_ref())
						} else {
							("I don't know how to respond.", ms.channel.as_ref())
						}
					},
					_ => ("", None)
				};
				if chan.is_some() {
					let _ = cli.sender().send_message(chan.as_ref().unwrap(), response_text);
				}
			},
			_ => println!("{:?}", event),
		};
	}

	fn on_close(&mut self, _cli: &RtmClient) {
		println!("on_close");
	}

	fn on_connect(&mut self, cli: &RtmClient) {
		println!("on_connect");
		// find the general channel id from the `StartResponse`
        let general_channel_id = cli.start_response()
            .channels
            .as_ref()
            .and_then(|channels| {
                          channels
                              .iter()
                              .find(|chan| match chan.name {
                                        None => false,
                                        Some(ref name) => name == "botspam",
                                    })
                      })
            .and_then(|chan| chan.id.as_ref())
            .expect("botspam channel not found");
        let _ = cli.sender().send_message(&general_channel_id, "Hello world! (rtm)");
        // Send a message over the real time api websocket
	}
}

fn main() {
	let args: Vec<String> = std::env::args().collect();
    let api_key = match args.len() {
        0 | 1 => panic!("No api-key in args! Usage: cargo run --example slack_example -- <api-key>"),
        x => args[x - 1].clone(),
    };
	let mut handler = MyHandler;
	let r = RtmClient::login_and_run(&api_key, &mut handler);
	match r {
		Ok(_) => (),
		Err(err) => panic!("Error: {}", err),
	}
}