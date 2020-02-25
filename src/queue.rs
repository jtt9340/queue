use std::{
	collections::VecDeque,
	fmt,
	iter::Extend,
};

use slack::RtmClient;

use crate::user::{
	SlackMap,
	UserID,
};

/// The User ID (a string of the form UXXXXXXX) for the Queue app
pub const QUEUE_UID: &str = "<@UQMDZF97S>";

/// Sometimes we need these.
pub const INSPIRATIONAL_QUOTE: &str =
	"_Waiting in line is a great opportunity to meet people, daydream, or play._\n\t\u{2014}Patch Adams";

/// Which Slack channel Queue is running in.
const CHANNEL: &str = "3d-printer-queue";

/// Given the body of a post to Slack, determine someone mentioned the Queue app
fn is_app_mention(text: &str) -> bool {
	text.contains(QUEUE_UID)
}

/// The main data structure for keeping track of Slack users for an event.
#[derive(Debug)]
// BIG TODO: Make implementation persistent (write to file)
pub struct Queue<'a> {
	/// A queue of references to UserIDs in the `uid_username_mapping`
	queue: VecDeque<UserID>,
	/// All the possible members of a Slack workspace that can join a queue
	uid_username_mapping: &'a SlackMap,
}

impl<'a> Queue<'a> {
	/// Create an empty queue. `uids_to_users` is a `std::collections::HashMap` whose keys are Slack
	/// IDs and whose values are usernames associated with the given Slack ID.
	pub fn new(uids_to_users: &'a SlackMap) -> Self {
		Self {
			queue: VecDeque::new(),
			uid_username_mapping: uids_to_users,
		}
	}

	/// Add a User to the back of the queue if he or she is not in line already.
	///
	/// Returns a reference to the user that has just been added, or None if that user was already in
	/// the queue.
	pub fn add_user(&mut self, user: UserID) -> Option<&UserID> {
		if self.queue.contains(&user) {
			None
		} else {
			self.queue.push_back(user);
			self.queue.back()
		}
	}

	/// Handle the add command. Returns a message to post in the Slack channel depending on whether
	/// or not the user was actually added.
	fn add(&mut self, user: UserID) -> String {
		match self.add_user(user) {
			Some(user) => format!("Okay <@{}>, I have added you to the queue.", user),
			None => String::from("You are already in the queue!"),
		}
	}

	/// Remove the person who is next in line for an event. Returns `None` if there is no such user,
	/// i.e. the queue is empty.
	pub fn remove_first_user_in_line(&mut self) -> Option<UserID> {
		self.queue.pop_front()
	}

	/// Handle the done command. Returns a message to post in the Slack channel depending on whether
	/// or not the user was removed.
	fn done(&mut self, user: UserID) -> String {
		if self
			.peek_first_user_in_line()
			.map_or(false, |u| *u == user)
		{
			/* It *should* be safe to unwrap() here because the condition ensures there is a
			first user in line in the first place */
			let user = self.remove_first_user_in_line().unwrap();
			let mut response = format!("Okay <@{}>, you have been removed from the front of the queue.", user.0);
			if let Some(next) = self.peek_first_user_in_line() {
				response.push_str(format!("\nHey <@{}>! You're next in line!", next.0).as_str());
			}
			response
		} else {
			String::from("You cannot be done; you are not at the front of the line!")
		}
	}

	/// Retrieve the person who is at the front if the line, if they exist. This does **not** remove
	/// the person, only retrieves them.
	///
	/// Returns `None` if the queue is empty. Else returns `Some(user)` where `user` is the user at
	/// the front of the line.
	pub(in crate) fn peek_first_user_in_line(&self) -> Option<&UserID> {
		self.queue.get(0)
	}

	/// Remove the particular user in the queue, e.g. if they no longer want to wait in line.
	///
	/// Returns `true` if the user was removed, `false` if the user wasn't, i.e. the user wasn't in
	/// queue to begin with.
	pub fn remove_user(&mut self, user: UserID) -> bool {
		// FIXME: this is kinda a naive implementation, perhaps a better implementation is in order?
		for idx in 0..self.queue.len() {
			if self.queue[idx] == user {
				self.queue.remove(idx);
				/* We can return early because it is invariant that there is only one of each user in
				the queue */
				return true;
			}
		}
		false
	}

	/// Given a Slack ID, return the real name-maybe username pair associated with that ID, if there is one.
	fn get_username_by_id(&self, id: &UserID) -> Option<&(Option<String>, Option<String>)> {
		self.uid_username_mapping.get(id)
	}

	/// Handle the cancel command. The cancel command will remove someone from the queue regardless
	/// of their position. The parameter `notify_next` is used to specify if the person behind the
	/// `user` who just left should be notified of this event. The string returned is the message to
	/// post in the Slack chat.
	fn cancel(&mut self, user: UserID) -> String {
		if user == *self
			.peek_first_user_in_line()
			.unwrap_or(&UserID::new(QUEUE_UID))
		{
			String::from("Use the done command to leave the queue. The difference \
			between cancel and done is that done will notify the next user in line.")
		} else if self.remove_user(user.clone()) {
			format!("Okay <@{}>, I have removed you from the queue.", user.0)
		} else {
			String::from("You weren't in the queue to begin with!")
		}
	}

	/// Given the `body` of what `user` posted when mentioning Queue, determine what to say back.
	///
	/// Currently, this function takes a **mutable reference** to `self` and has the side-effect of
	/// mutating `self`. In the future, it might return another value indicating how to mutate queue
	/// after invocation of this method.
	pub fn determine_response(&mut self, user: UserID, body: &str) -> String {
		/*
			Commands are only activated when the body has an @Queue. But we need to strip the command
			of its @Queue mention before seeing what the user wants Queue to do.
		*/
		// TODO: handle cases where the mention is not at the beginning of the string
		// also TODO: Merge done and cancel into one command
		let lowercase_queue_id = QUEUE_UID.to_lowercase();
		let body = body.to_lowercase();
		let body = body.trim_start_matches(lowercase_queue_id.as_str());

		match body.trim() {
			"add" => self.add(user),
			"cancel" => self.cancel(user),
			"done" => self.done(user),
			"show" => format!("{}", self),
			s => format!("Unrecognized command {}. Your options are: add, cancel, done, and show", s)
		}
	}
}

impl slack::EventHandler for Queue<'_> {
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
							UserID(user),
							text.as_str(),
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
		println!("{}", INSPIRATIONAL_QUOTE);
		let botspam_chan_id = cli
			.start_response()
			.channels
			.as_ref()
			.and_then(|chans| {
				chans
					.iter()
					.find(|chan| match chan.name {
						None => false,
						Some(ref name) => name == CHANNEL,
					})
			})
			.and_then(|chan| chan.id.as_ref())
			.expect("channel botspam not found")
		;
		let _ = cli.sender().send_message(&botspam_chan_id, INSPIRATIONAL_QUOTE);
	}
}

impl Extend<UserID> for Queue<'_> {
	fn extend<T>(&mut self, iter: T) where T: IntoIterator<Item=UserID> {
		self.queue.extend(iter);
	}
}

impl fmt::Display for Queue<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Here are the people currently in line:\n{}", self
			.queue
			.iter()
			.enumerate()
			.map(|idx_user_pair| {
				let (idx, u) = idx_user_pair;
				let (maybe_real_name, maybe_username) = self
					.get_username_by_id(u)
					.expect(format!("For some reason user {} did not have an ID", u).as_str())
					;
				let u = &u.to_string();
				let real_name = maybe_real_name.as_ref().unwrap_or(u);
				match maybe_username {
					Some(uname) if !uname.is_empty() => format!("{}. {} ({})\n", idx, real_name, uname),
					_ => format!("{}. {}\n", idx, real_name),
				}
			})
			.fold(String::default(), |acc, line| acc.to_owned() + &line)
		)
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use super::*;

	#[test]
	fn create_queue() {
		let hash_map = HashMap::new();
		let queue_a = Queue::new(&hash_map);
		let queue_b = Queue {
			queue: VecDeque::new(),
			uid_username_mapping: &hash_map,
		};

		assert_eq!(queue_a.queue, queue_b.queue);
		assert_eq!(queue_a.uid_username_mapping, queue_b.uid_username_mapping);
	}

	#[test]
	fn add_users() {
		let hash_map = HashMap::new();
		let mut queue = Queue::new(&hash_map);

		assert_eq!(queue.add_user(UserID::new("UA8RXUPSP")), Some(&UserID::new("UA8RXUPSP")));
		assert_eq!(queue.add_user(UserID::new("UNB2LMZRP")), Some(&UserID::new("UNB2LMZRP")));
		assert_eq!(queue.add_user(UserID::new("UN480W9ND")), Some(&UserID::new("UN480W9ND")));

		assert_eq!(queue.queue, [
			UserID::new("UA8RXUPSP"),
			UserID::new("UNB2LMZRP"),
			UserID::new("UN480W9ND"),
		]);
	}

	#[test]
	fn add_duplicate_users() {
		let hash_map = HashMap::new();
		let mut queue = Queue::new(&hash_map);

		assert_eq!(queue.add_user(UserID::new("UA8RXUPSP")), Some(&UserID::new("UA8RXUPSP")));
		assert_eq!(queue.add_user(UserID::new("UNB2LMZRP")), Some(&UserID::new("UNB2LMZRP")));
		assert!(queue.add_user(UserID::new("UA8RXUPSP")).is_none());

		assert_eq!(queue.queue, [
			UserID::new("UA8RXUPSP"),
			UserID::new("UNB2LMZRP"),
		]);
	}

	#[test]
	fn remove_front_users() {
		let hash_map = HashMap::new();
		let mut queue = Queue::new(&hash_map);

		queue.add_user(UserID::new("UA8RXUPSP"));
		queue.add_user(UserID::new("UNB2LMZRP"));
		queue.add_user(UserID::new("UN480W9ND"));

		assert_eq!(queue.remove_first_user_in_line(), Some(UserID::new("UA8RXUPSP")));
		assert_eq!(queue.queue, [
			UserID::new("UNB2LMZRP"),
			UserID::new("UN480W9ND"),
		]);

		// Empty the queue
		queue.remove_first_user_in_line();
		queue.remove_first_user_in_line();

		assert_eq!(None, queue.remove_first_user_in_line());
		assert_eq!(queue.queue, []);
	}

	#[test]
	fn peek_front_users() {
		let hash_map = HashMap::new();
		let mut queue = Queue::new(&hash_map);

		queue.add_user(UserID::new("UA8RXUPSP"));
		queue.add_user(UserID::new("UNB2LMZRP"));
		queue.add_user(UserID::new("UN480W9ND"));

		assert_eq!(queue.peek_first_user_in_line(), Some(&UserID::new("UA8RXUPSP")));
		// Does not mutate the queue itself
		assert_eq!(queue.queue, [
			UserID::new("UA8RXUPSP"),
			UserID::new("UNB2LMZRP"),
			UserID::new("UN480W9ND"),
		]);
	}

	#[test]
	fn remove_arbitrary_users() {
		let hash_map = HashMap::new();
		let mut queue = Queue::new(&hash_map);

		queue.add_user(UserID::new("UA8RXUPSP"));
		queue.add_user(UserID::new("UNB2LMZRP"));
		queue.add_user(UserID::new("UN480W9ND"));

		assert!(queue.remove_user(UserID::new("UNB2LMZRP")));
		assert_eq!(queue.queue, [
			UserID::new("UA8RXUPSP"),
			UserID::new("UN480W9ND"),
		]);
	}

	#[test]
	fn remove_non_existent_users() {
		let hash_map = HashMap::new();
		let mut queue = Queue::new(&hash_map);

		assert!(!queue.remove_user(UserID::new("UNB2LMZRP")));

		queue.add_user(UserID::new("UA8RXUPSP"));
		queue.add_user(UserID::new("UNB2LMZRP"));
		queue.add_user(UserID::new("UN480W9ND"));

		queue.remove_first_user_in_line();
		assert!(!queue.remove_user(UserID::new("UA8RXUPSP")));
	}

	#[test]
	fn extend_queue() {
		let hash_map = HashMap::new();
		let mut queue = Queue::new(&hash_map);

		queue.extend(vec![
			UserID::new("UA8RXUPSP"),
			UserID::new("UNB2LMZRP"),
			UserID::new("UN480W9ND"),
		]);

		assert_eq!(queue.queue, [
			UserID::new("UA8RXUPSP"),
			UserID::new("UNB2LMZRP"),
			UserID::new("UN480W9ND"),
		]);
	}
}