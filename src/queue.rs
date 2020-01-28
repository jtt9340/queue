use std::{
	collections::VecDeque,
	convert::From,
	fmt,
	iter::Extend,
};

use slack::RtmClient;

use crate::user::User;

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
pub struct Queue(VecDeque<User>);

impl Queue {
	/// Create an empty queue.
	pub fn new() -> Self {
		Self(VecDeque::new())
	}

	/// Add a User to the back of the queue if he or she is not in line already.
	///
	/// Returns whether or not the user was added to the queue. If they weren't, it's because they are
	/// already in the queue.
	pub fn add_user(&mut self, user: User) -> bool {
		if self.0.contains(&user) {
			false
		} else {
			self.0.push_back(user);
			true
		}
	}

	/// Handle the add command. Returns a message to post in the Slack channel depending on whether
	/// or not the user was actually added.
	fn add(&mut self, user: User) -> String {
		if self.add_user(user.clone()) {
			format!("Okay <@{}>, I have added you to the queue", user.0)
		} else {
			String::from("You are already in the queue!")
		}
	}

	/// Remove the person who is next in line for an event. Returns `None` if there is no such user,
	/// i.e. the queue is empty.
	pub fn remove_first_user_in_line(&mut self) -> Option<User> {
		self.0.pop_front()
	}

	/// Handle the done command. Returns a message to post in the Slack channel depending on whether
	/// or not the user was removed.
	fn done(&mut self, user: User) -> String {
		if self
			.peek_first_user_in_line()
			.map_or(false, |u| *u == user)
		{
			/* It *should* be safe to unwrap() here because the condition ensures there is a
			first user in line in the first place */
			let user = self.remove_first_user_in_line().unwrap();
			let mut response = format!("Okay <@{}>, you have been removed from the front of the queue.", user.0);
			if let Some(next) = self.peek_first_user_in_line() {
				response.push_str(format!("Hey <@{}>! You're next in line!", next.0).as_str());
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
	pub(in crate) fn peek_first_user_in_line(&self) -> Option<&User> {
		self.0.get(0)
	}

	/// Remove the particular user in the queue, e.g. if they no longer want to wait in line.
	///
	/// Returns `true` if the user was removed, `false` if the user wasn't, i.e. the user wasn't in
	/// queue to begin with.
	pub fn remove_user(&mut self, user: User) -> bool {
		// FIXME: this is kinda a naive implementation, perhaps a better implementation is in order?
		for idx in 0..self.0.len() {
			if self.0[idx] == user {
				self.0.remove(idx);
				/* We can return early because it is invariant that there is only one of each user in
				the queue */
				return true;
			}
		}
		false
	}

	/// Handle the cancel command. The cancel command will remove someone from the queue regardless
	/// of their position. The parameter `notify_next` is used to specify if the person behind the
	/// `user` who just left should be notified of this event. The string returned is the message to
	/// post in the Slack chat.
	fn cancel(&mut self, user: User) -> String {
		if user == *self.peek_first_user_in_line().unwrap_or(&User(QUEUE_UID.to_string())) {
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
	pub fn determine_response(&mut self, user: User, body: &str) -> String {
		/*
			Commands are only activated when the body has an @Queue. But we need to strip the command
			of its @Queue mention before seeing what the user wants Queue to do.
		*/
		let lowercase_queue_id = QUEUE_UID.to_lowercase();
		let body = body.to_lowercase();
		let body = body.trim_start_matches(lowercase_queue_id.as_str());
		
		match body.trim() {
			"add" => self.add(user),
			"cancel" => self.cancel(user),
			"done" => self.done(user),
			"show" => /* TODO: Current implementation will mention users */ format!("{}", self),
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
							User(String::from(user)),
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

impl Extend<User> for Queue {
	fn extend<T>(&mut self, iter: T) where T: IntoIterator<Item=User> {
		self.0.extend(iter);
	}
}

impl From<VecDeque<User>> for Queue {
	fn from(vec_deque: VecDeque<User>) -> Self {
		Self(vec_deque)
	}
}

impl From<Vec<User>> for Queue {
	fn from(vec: Vec<User>) -> Self {
		Self(VecDeque::from(vec))
	}
}

impl fmt::Display for Queue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Here are the people currently in line:\n{}", self
			.0
			.iter()
			.map(|u| format!("• <@{}>\n", u.0))
			.fold(String::default(), |acc, line| acc.to_owned() + &line)
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn create_queue() {
		assert_eq!(Queue::new().0, []);
	}

	#[test]
	fn add_users() {
		let mut queue = Queue::new();

		assert!(queue.add_user(User(String::from("UA8RXUPSP"))));
		assert!(queue.add_user(User(String::from("UNB2LMZRP"))));
		assert!(queue.add_user(User(String::from("UN480W9ND"))));

		assert_eq!(queue.0, [User(String::from("UA8RXUPSP")), User(String::from("UNB2LMZRP")), User(String::from("UN480W9ND"))]);
	}

	#[test]
	fn add_duplicate_users() {
		let mut queue = Queue::new();

		assert!(queue.add_user(User(String::from("UA8RXUPSP"))));
		assert!(queue.add_user(User(String::from("UNB2LMZRP"))));
		assert!(!queue.add_user(User(String::from("UA8RXUPSP"))));

		assert_eq!(queue.0, [User(String::from("UA8RXUPSP")), User(String::from("UNB2LMZRP"))]);
	}

	#[test]
	fn remove_front_users() {
		let mut queue = Queue::new();

		queue.add_user(User(String::from("UA8RXUPSP")));
		queue.add_user(User(String::from("UNB2LMZRP")));
		queue.add_user(User(String::from("UN480W9ND")));

		assert_eq!(queue.remove_first_user_in_line(), Some(User(String::from("UA8RXUPSP"))));
		assert_eq!(queue.0, [User(String::from("UNB2LMZRP")), User(String::from("UN480W9ND"))]);

		// Empty the queue
		queue.remove_first_user_in_line();
		queue.remove_first_user_in_line();

		assert_eq!(None, queue.remove_first_user_in_line());
		assert_eq!(queue.0, []);
	}

	#[test]
	fn peek_front_users() {
		let mut queue = Queue::new();

		queue.add_user(User(String::from("UA8RXUPSP")));
		queue.add_user(User(String::from("UNB2LMZRP")));
		queue.add_user(User(String::from("UN480W9ND")));

		assert_eq!(queue.peek_first_user_in_line(), Some(&User(String::from("UA8RXUPSP"))));
		// Does not mutate the queue itself
		assert_eq!(queue.0, [User(String::from("UA8RXUPSP")), User(String::from("UNB2LMZRP")), User(String::from("UN480W9ND"))]);
	}

	#[test]
	fn remove_arbitrary_users() {
		let mut queue = Queue::new();

		queue.add_user(User(String::from("UA8RXUPSP")));
		queue.add_user(User(String::from("UNB2LMZRP")));
		queue.add_user(User(String::from("UN480W9ND")));

		assert!(queue.remove_user(User(String::from("UNB2LMZRP"))));
		assert_eq!(queue.0, [User(String::from("UA8RXUPSP")), User(String::from("UN480W9ND"))]);
	}

	#[test]
	fn remove_non_existent_users() {
		let mut queue = Queue::new();

		assert!(!queue.remove_user(User(String::from("UNB2LMZRP"))));

		queue.add_user(User(String::from("UA8RXUPSP")));
		queue.add_user(User(String::from("UNB2LMZRP")));
		queue.add_user(User(String::from("UN480W9ND")));

		queue.remove_first_user_in_line();
		assert!(!queue.remove_user(User(String::from("UA8RXUPSP"))));
	}

	#[test]
	fn extend_queue() {
		let mut queue = Queue::new();

		queue.extend(vec![
			User(String::from("UA8RXUPSP")),
			User(String::from("UNB2LMZRP")),
			User(String::from("UN480W9ND")),
		]);

		assert_eq!(queue.0, [User(String::from("UA8RXUPSP")), User(String::from("UNB2LMZRP")), User(String::from("UN480W9ND"))]);
	}

	#[test]
	fn from_vec_deque() {
		let mut vec_deque = VecDeque::new();
		vec_deque.push_back(User(String::from("UA8RXUPSP")));
		vec_deque.push_back(User(String::from("UNB2LMZRP")));
		vec_deque.push_back(User(String::from("UN480W9ND")));

		let queue = Queue::from(vec_deque);
		assert_eq!(queue.0, [User(String::from("UA8RXUPSP")), User(String::from("UNB2LMZRP")), User(String::from("UN480W9ND"))]);
	}

	#[test]
	fn from_vec() {
		let queue = Queue::from(vec![
			User(String::from("UA8RXUPSP")),
			User(String::from("UNB2LMZRP")),
			User(String::from("UN480W9ND")),
		]);

		assert_eq!(queue.0, [User(String::from("UA8RXUPSP")), User(String::from("UNB2LMZRP")), User(String::from("UN480W9ND"))]);
	}
}