/// The User ID (a string of the form UXXXXXXX) for the Queue app
pub const QUEUE_UID: &str = "<@UQMDZF97S>";

/// A user of Slack, i.e. someone who will wait in line for an event.
///
/// This type simply wraps a string of the format UXXXXXXXX which represents the ID of a Slack user.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct User {
	/// The Slack user ID in the format UXXXXXXX
	pub(crate) uid: String,
	/// The user name of the user (e.x. "Joe Smith")
	pub(crate) username: String,
}