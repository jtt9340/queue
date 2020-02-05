use std::{collections::HashMap, env, process};
use serde::Deserialize;

pub use print_queue::queue;
pub use print_queue::user;

/// The shape of the JSON returned by the Slack users.list method.
#[derive(Debug, Deserialize)]
struct UsersList {
	cache_ts: u32,
	members: Vec<slack::User>,
	ok: bool,
	response_metadata: ResponseMetadata,
}

/// The JSON object part of the JSON returned by the Slack users.list method.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
struct ResponseMetadata {
	next_cursor: String,
}

fn create_uid_username_mapping(auth_token: &str) -> reqwest::Result<HashMap<String, String>> {
	// I'M GONNA GET THE REAL NAMES FINALLY!!!!!1
	let client = reqwest::blocking::Client::new();
	let users = client
		.get("https://slack.com/api/users.list") // reqwest::RequestBuilder
		.bearer_auth(auth_token) // reqwest::RequestBuilder
		.send()? // reqwest::blocking::response::Response
		.json::<UsersList>()? // UsersList
	;

	// Yikes there are about 772 users
	let mut uid_username_mapping = HashMap::with_capacity(780);

	for user in users.members {
		/* id lives at slack::User.id
		   username lives at slack::User.profile.display_name */
		let id = if let Some(id) = user.id {
			id
		} else {
			panic!("This user does not have an id: {:#?}", user);
		};
		/*
			If, for some reason, we are unable to to get the user's username, we will fall back to
			their ID. TODO: Fall back to real name and then finally ID?
			There are three reasons we would fall back to their ID:
				1) The user does not have a profile (i.e. user.profile == None)
				2) The user has a profile, but they do not have a display name
				3) The user has a profile and a display name, but the display name is the empty string
		*/
		let display_name = match user.profile {
			Some(prof) => match prof.display_name {
				Some(name) => if name.is_empty() {
					id.clone() // Case 3)
				} else {
					name
				},
				None => id.clone(), // Case 2)
			},
			None => id.clone(), // Case 1)
		};
		uid_username_mapping.insert(id, display_name);
	}

	Ok(uid_username_mapping)
}

/// Entry point for the Slack bot.
fn main() -> Result<(), slack::error::Error> {
	let args = env::args().collect::<Vec<String>>();
	let api_key = match args.len() {
		0 | 1 => {
			eprintln!("No API key in args! Usage: cargo run --bin slack_main --features slack -- <api-key>");
			process::exit(-1);
		},
		nargs => args[nargs - 1].clone(),
	};

	let users = match create_uid_username_mapping(api_key.as_str()) {
		Ok(u) => u,
		Err(e) => {
			eprintln!("{}", e);
			process::exit(-2);
		},
	};
	println!("{:#?}", users);
	println!("Number of members: {:?}", users.len());

	let mut queue = queue::Queue::new(users);
	slack::RtmClient::login_and_run(&api_key, &mut queue)
}