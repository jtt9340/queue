/// A user of Slack, i.e. someone who will wait in line for an event.
///
/// This type simply wraps a string of the format UXXXXXXXX which represents the ID of a Slack user.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct User(pub String);