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
// TODO: Make implementation persistent (write to file)
pub struct Queue {
	queue: VecDeque<UserID>,
	uid_username_mapping: SlackMap,
}

impl Queue {
	/// Create an empty queue. `uids_to_users` is a `std::collections::HashMap` whose keys are Slack
	/// IDs and whose values are usernames associated with the given Slack ID.
	pub fn new(uids_to_users: SlackMap) -> Self {
		Self {
			queue: VecDeque::new(),
			uid_username_mapping: uids_to_users,
		}
	}

	/// Add a User to the back of the queue if he or she is not in line already.
	///
	/// Returns whether or not the user was added to the queue. If they weren't, it's because they are
	/// already in the queue.
	pub fn add_user(&mut self, user: UserID) -> bool {
		if self.queue.contains(&user) {
			false
		} else {
			self.queue.push_back(user);
			true
		}
	}

	/// Handle the add command. Returns a message to post in the Slack channel depending on whether
	/// or not the user was actually added.
	fn add(&mut self, user: UserID) -> String {
		if self.add_user(user.clone()) {
			format!("Okay <@{}>, I have added you to the queue", user.uid())
		} else {
			String::from("You are already in the queue!")
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
			let mut response = format!("Okay <@{}>, you have been removed from the front of the queue.", user.uid());
			if let Some(next) = self.peek_first_user_in_line() {
				response.push_str(format!("Hey <@{}>! You're next in line!", next.uid()).as_str());
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
	fn get_username_by_id(&self, id: &str) -> Option<&(String, Option<String>)> {
		self.uid_username_mapping.get(id)
	}

	/// Handle the cancel command. The cancel command will remove someone from the queue regardless
	/// of their position. The parameter `notify_next` is used to specify if the person behind the
	/// `user` who just left should be notified of this event. The string returned is the message to
	/// post in the Slack chat.
	fn cancel(&mut self, user: UserID) -> String {
		if user == *self
			.peek_first_user_in_line()
			.unwrap_or(&UserID::new(QUEUE_UID, 0 /* TODO: CHANGE ORDINAL */))
		{
			String::from("Use the done command to leave the queue. The difference \
			between cancel and done is that done will notify the next user in line.")
		} else if self.remove_user(user.clone()) {
			format!("Okay <@{}>, I have removed you from the queue.", user.uid())
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

impl slack::EventHandler for Queue {
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
							/* TODO: change determine_response to only accept &strs and then create a
								    user in determine_response so that its ordinal position is correct */
							UserID::new(&*user, 0),
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

impl Extend<UserID> for Queue {
	fn extend<T>(&mut self, iter: T) where T: IntoIterator<Item=UserID> {
		self.queue.extend(iter);
	}
}

impl fmt::Display for Queue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Here are the people currently in line:\n{}", self
			.queue
			.iter()
			.map(|u| {
				let (real_name_or_id, maybe_username) = self
					.get_username_by_id(u.uid().as_str())
					.expect(format!("For some reason user {:?} did not have a real name", u)
						.as_str()
					)
				;
				format!("• {}{}\n", real_name_or_id, match maybe_username {
					Some(uname) if !uname.is_empty() => format!(" ({})", uname),
					_ => String::default(),
				})
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
		let queue_a = Queue::new(HashMap::new());
		let queue_b = Queue {
			queue: VecDeque::new(),
			uid_username_mapping: HashMap::new(),
		};

		assert_eq!(queue_a.queue, queue_b.queue);
		assert_eq!(queue_a.uid_username_mapping, queue_b.uid_username_mapping);
	}

	#[test]
	fn add_users() {
		let mut queue = Queue::new(HashMap::new());

		assert!(queue.add_user(UserID::new("UA8RXUPSP", 0)));
		assert!(queue.add_user(UserID::new("UNB2LMZRP", 1)));
		assert!(queue.add_user(UserID::new("UN480W9ND", 2)));

		assert_eq!(queue.queue, [
			UserID::new("UA8RXUPSP", 0),
			UserID::new("UNB2LMZRP", 1),
			UserID::new("UN480W9ND", 2),
		]);
	}

	#[test]
	fn add_duplicate_users() {
		let mut queue = Queue::new(HashMap::new());

		assert!(queue.add_user(UserID::new("UA8RXUPSP", 0)));
		assert!(queue.add_user(UserID::new("UNB2LMZRP", 1)));
		assert!(!queue.add_user(UserID::new("UA8RXUPSP", 2)));

		assert_eq!(queue.queue, [
			UserID::new("UA8RXUPSP", 0),
			UserID::new("UNB2LMZRP", 1),
		]);
	}

	#[test]
	fn remove_front_users() {
		let mut queue = Queue::new(HashMap::new());

		queue.add_user(UserID::new("UA8RXUPSP", 0));
		queue.add_user(UserID::new("UNB2LMZRP", 1));
		queue.add_user(UserID::new("UN480W9ND", 2));

		assert_eq!(queue.remove_first_user_in_line(), Some(UserID::new("UA8RXUPSP", 0)));
		assert_eq!(queue.queue, [
			UserID::new("UNB2LMZRP", 1),
			UserID::new("UN480W9ND", 2),
		]);

		// Empty the queue
		queue.remove_first_user_in_line();
		queue.remove_first_user_in_line();

		assert_eq!(None, queue.remove_first_user_in_line());
		assert_eq!(queue.queue, []);
	}

	#[test]
	fn peek_front_users() {
		let mut queue = Queue::new(HashMap::new());

		queue.add_user(UserID::new("UA8RXUPSP", 0));
		queue.add_user(UserID::new("UNB2LMZRP", 1));
		queue.add_user(UserID::new("UN480W9ND", 2));

		assert_eq!(queue.peek_first_user_in_line(), Some(&UserID::new("UA8RXUPSP", 0)));
		// Does not mutate the queue itself
		assert_eq!(queue.queue, [
			UserID::new("UA8RXUPSP", 0),
			UserID::new("UNB2LMZRP", 1),
			UserID::new("UN480W9ND", 2),
		]);
	}

	#[test]
	fn remove_arbitrary_users() {
		let mut queue = Queue::new(HashMap::new());

		queue.add_user(UserID::new("UA8RXUPSP", 0));
		queue.add_user(UserID::new("UNB2LMZRP", 1));
		queue.add_user(UserID::new("UN480W9ND", 2));

		assert!(queue.remove_user(UserID::new("UNB2LMZRP", 1)));
		assert_eq!(queue.queue, [
			UserID::new("UA8RXUPSP", 0),
			UserID::new("UN480W9ND", 2),
		]);
	}

	#[test]
	fn remove_non_existent_users() {
		let mut queue = Queue::new(HashMap::new());

		assert!(!queue.remove_user(UserID::new("UNB2LMZRP", 0)));

		queue.add_user(UserID::new("UA8RXUPSP", 0));
		queue.add_user(UserID::new("UNB2LMZRP", 1));
		queue.add_user(UserID::new("UN480W9ND", 2));

		queue.remove_first_user_in_line();
		assert!(!queue.remove_user(UserID::new("UA8RXUPSP", 0)));
	}

	#[test]
	fn extend_queue() {
		let mut queue = Queue::new(HashMap::new());

		queue.extend(vec![
			UserID::new("UA8RXUPSP", 0),
			UserID::new("UNB2LMZRP", 1),
			UserID::new("UN480W9ND", 2),
		]);

		assert_eq!(queue.queue, [
			UserID::new("UA8RXUPSP", 0),
			UserID::new("UNB2LMZRP", 1),
			UserID::new("UN480W9ND", 2),
		]);
	}

	/* Queue no longer implements From<VecDeque<UserID>> and From<Vec<UserID>>
	#[test]
	fn from_vec_deque() {
		let mut vec_deque = VecDeque::new();
		vec_deque.push_back(UserID::new("UA8RXUPSP", 0));
		vec_deque.push_back(UserID::new("UNB2LMZRP", 1));
		vec_deque.push_back(UserID::new("UN480W9ND", 2));

		let queue = Queue::from(vec_deque);
		assert_eq!(queue.0, [UserID(String::from("UA8RXUPSP")), UserID(String::from("UNB2LMZRP")), UserID(String::from("UN480W9ND"))]);
	}

	#[test]
	fn from_vec() {
		let queue = Queue::from(vec![
			UserID(String::from("UA8RXUPSP")),
			UserID(String::from("UNB2LMZRP")),
			UserID(String::from("UN480W9ND")),
		]);

		assert_eq!(queue.0, [UserID(String::from("UA8RXUPSP")), UserID(String::from("UNB2LMZRP")), UserID(String::from("UN480W9ND"))]);
	}*/
}