use std::collections::HashMap;
use serde::Deserialize;

/// A user of Slack, i.e. someone who will wait in line for an event.
///
/// This type simply wraps a string of the format UXXXXXXXX which represents the ID of a Slack user.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct User(pub String);

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

/// A mapping from Slack user IDs to real name-maybe username pairs.
pub type SlackMap = HashMap<String, (String, Option<String>)>;

pub fn create_uid_username_mapping(auth_token: &str) -> reqwest::Result<SlackMap> {
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

	println!("{:#?}", users.members[70]);
	for user in users.members {
		/* id lives at slack::User.id
		   username lives at slack::User.profile.display_name */
		let id = if let Some(id) = user.id {
			id
		} else {
			panic!("This user does not have an id: {:#?}", user);
		};
		/*
			If, for some reason, we are unable to to get the user's username and real name, we will
			fall back to their ID. TODO: Fall back to real name and then finally ID?
			There are three reasons we would fall back to their ID:
				1) The user does not have a profile (i.e. user.profile == None)
				2) The user has a profile, but they do not have a display name
				3) The user has a profile and a display name, but the display name is the empty string
		*/
		/*let username: Option<String> = match user.profile {
			Some(prof) => match prof.display_name {
				Some(name) => if name.is_empty() {
					None // Case 3)
				} else {
					name
				},
				None => None, // Case 2)
			},
			None => None, // Case 1)
		};*/

		let real_name = user
			.profile
			.as_ref()
			.and_then(|prof| prof.real_name.clone())
			.unwrap_or(id.clone())
		;

		let username = user
			.profile
			.and_then(|prof| prof.display_name)
		;

		uid_username_mapping.insert(id, (real_name, username));
	}

	Ok(uid_username_mapping)
}