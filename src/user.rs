use std::{
	cmp,
	collections::HashMap,
	fmt,
};

use serde::Deserialize;

/// A user of Slack, i.e. someone who will wait in line for an event.
///
/// This type simply wraps a string of the format UXXXXXXXX, which represents the ID of a Slack user,
/// and stores a user's position in line.
#[derive(Clone)]
pub struct UserID {
	/// A string of the form UXXXXXXXX representing the ID of a Slack user.
	uid: String,
	/// The number position this user in is waiting in line.
	position: u16,
}

impl UserID {
	/// Create a new UserID with a given `user_id` and `ordinal` position in line.
	///
	/// This function does not parse `user_id` to ensure it is a valid user ID, since the exact format
	/// of a valid user ID is currently unknown. At some point, this function may do such parsing and
	/// thus return an Option<Self>, depending on if the ID passed in could not be parsed as a valid
	/// ID.
	pub fn new(user_id: &str, ordinal: u16) -> Self {
		Self {
			uid: user_id.to_string(),
			position: ordinal,
		}
	}

	/// Get (an immutable reference to) the user ID of `self`.
	pub fn uid(&self) -> &String {
		&self.uid
	}

	/// Get the position in line that this User ID is in.
	pub fn position(&self) -> u16 {
		self.position
	}
}

impl fmt::Debug for UserID {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		/*fmt.debug_tuple("User")
			.field(&self.uid)
			.field(&self.position)
			.finish()*/
		write!(fmt, "User({}, {})", self.uid, self.position)
	}
}

impl fmt::Display for UserID {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt.write_str(&*self.uid)
	}
}

impl cmp::PartialEq for UserID {
	/// Compares two UserIDs by **ID only**. Two UserIDs can have the same UserID but different
	/// positions and still be considered equal. This is so that the same Slack user cannot be added
	/// to a queue in two different positions.
	fn eq(&self, other: &UserID) -> bool {
		self.uid == other.uid
	}
}

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
			We seek to get both the real name and username from the Slack response. If we cannot get
			a user's real name, we default to the User ID. If we cannot get their username, we will
			simply not display their username.
		*/
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn create_user_id() {
		let user_a = UserID::new("UA8RXUPSP", 1);
		let user_b = UserID {
			uid: String::from("UA8RXUPSP"),
			position: 1,
		};

		/* We cannot do assert_eq!(user_a, user_b) because it relies on the implementation of PartialEq
		which, for UserID, does **not** do a field-by-field comparison. */
		assert_eq!(user_a.uid, user_b.uid);
		assert_eq!(user_a.position, user_b.position);
	}

	#[test]
	fn uid_accessor() {
		let user = UserID {
			uid: String::from("UA8RXUPSP"),
			position: 1,
		};

		assert_eq!(&user.uid, user.uid());
	}

	#[test]
	fn position_accessor() {
		let user = UserID {
			uid: String::from("UA8RXUPSP"),
			position: 1,
		};

		assert_eq!(user.position, user.position());
	}

	#[test]
	fn debug() {
		let user = UserID {
			uid: String::from("UA8RXUPSP"),
			position: 1,
		};

		assert_eq!(
			format!("User({}, {})", user.uid, user.position),
			format!("{:?}", user)
		);
	}

	#[test]
	fn display() {
		let user = UserID {
			uid: String::from("UA8RXUPSP"),
			position: 1,
		};

		assert_eq!(format!("{}", user), user.uid);
	}

	#[test]
	fn partial_eq_same_uid_same_position() {
		let user_a = UserID {
			uid: String::from("UA8RXUPSP"),
			position: 0,
		};

		let user_b = UserID {
			uid: String::from("UA8RXUPSP"),
			position: 0,
		};

		assert_eq!(user_a, user_b);
	}

	#[test]
	fn partial_eq_same_uid_different_position() {
		let user_a = UserID {
			uid: String::from("UA8RXUPSP"),
			position: 0,
		};

		let user_b = UserID {
			uid: String::from("UA8RXUPSP"),
			position: 1,
		};

		assert_eq!(user_a, user_b);
	}

	#[test]
	fn partial_eq_different_uid_same_position() {
		let user_a = UserID {
			uid: String::from("UA8RXUPSP"),
			position: 0,
		};

		let user_b = UserID {
			uid: String::from("UNB2LMZRP"),
			position: 0,
		};

		assert_ne!(user_a, user_b);
	}

	#[test]
	fn partial_eq_different_uid_different_position() {
		let user_a = UserID {
			uid: String::from("UA8RXUPSP"),
			position: 0,
		};

		let user_b = UserID {
			uid: String::from("UNB2LMZRP"),
			position: 1,
		};

		assert_ne!(user_a, user_b);
	}
}