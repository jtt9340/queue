use std::{
	collections::VecDeque,
	convert::From,
	iter::Extend,
};
use crate::user::User;

/// The User ID (a string of the form UXXXXXXX) for the Queue app
pub const QUEUE_UID: &str = "<@UQMDZF97S>";

/// The main data structure for keeping track of Slack users for an event.
#[derive(Debug)]
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

	/// Remove the person who is next in line for an event. Returns `None` if there is no such user,
	/// i.e. the queue is empty.
	pub fn remove_first_user_in_line(&mut self) -> Option<User> {
		self.0.pop_front()
	}

	/// Retrieve the person who is at the front if the line, if they exist. This does **not** remove
	/// the person, only retrieves them.
	///
	/// Returns `None` if the queue is empty. Else returns `Some(user)` where `user` is the user at
	/// the front of the line.
	pub fn peek_first_user_in_line(&self) -> Option<&User> {
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
			// TODO: Move each of these into their own method
			"add" => {
				if self.add_user(user.clone()) {
					format!("Okay <@{}>, I have added you to the queue", user.0)
				} else {
					String::from("You are already in the queue!")
				}
			},
			"cancel" => {
				// TODO: Prevent user from cancelling if they are at the front of the line
				if self.remove_user(user.clone()) {
					format!("Okay <@{}>, I have removed you from the queue", user.0)
				} else {
					String::from("You weren't in the queue to begin with!")
				}
			},
			"done" => {
				// TODO: Notify user when they are next in line
				if self
					.peek_first_user_in_line()
					.map_or(false, |u| *u == user)
				{
					/* It *should* be safe to unwrap() here because the condition ensures there is a
					first user in line in the first place */
					let user = self.remove_first_user_in_line().unwrap();
					format!("Okay <@{}>, you have been removed from the front of the queue.", user.0)
				} else {
					String::from("You cannot be done; you are not at the front of the line!")
				}
			},
			"show" => /* TODO: Use user names instead of User IDs */ format!("{:?}", self),
			s => format!("Unrecognized command {}. Your options are: add, cancel, done, and show", s)
		}
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