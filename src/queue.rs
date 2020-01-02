use std::collections::VecDeque;

/// A user of Slack, i.e. someone who will wait in line for an event.
///
/// This type simply wraps a string of the format UXXXXXXXX which represents the ID of a Slack user.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct User(pub String);

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

	/// Remove the particular user in the queue, e.g. if they no longer want to wait in line.
	///
	/// Returns `true` if the user was removed, `false` if the user wasn't, i.e. the user wasn't in
	/// queue to begin with.
	pub fn remove_user(&mut self, user: User) -> bool {
		// TODO: this is kinda a naive implementation, perhaps a better implementation is in order?
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
}