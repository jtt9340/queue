use std::{collections::HashMap, fmt};

use serde::Deserialize;

/// A user of Slack, i.e. someone who will wait in line for an event.
///
/// This type simply wraps a string of the format UXXXXXXXX which represents the ID of a Slack user.
#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct UserID(pub String);

impl UserID {
    /// Create a new UserID with a given `user_id`.
    ///
    /// This function does not parse `user_id` to ensure it is a valid user ID, since the exact format
    /// of a valid user ID is currently unknown. At some point, this function may do such parsing and
    /// thus return an Option<Self>, depending on if the ID passed in could not be parsed as a valid
    /// ID.
    pub fn new(user_id: &str) -> Self {
        Self(user_id.to_string())
    }
}

impl fmt::Display for UserID {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(self.0.as_str())
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
// TODO: Create struct so that we don't have to use (Option<String>, Option<String>)
pub type SlackMap = HashMap<UserID, (Option<String>, Option<String>)>;

/// Given a Slack bot authentication token, create a `std::collections::HashMap` that maps Slack user
/// IDs to a pair. The first item in the pair is the corresponding Slack user's real name, if one is
/// set in Slack. The second item in the pair is the corresponding Slack user's username, if one is
/// set in Slack.
pub fn create_uid_username_mapping(auth_token: &str) -> reqwest::Result<SlackMap> {
    // I'M GONNA GET THE REAL NAMES FINALLY!!!!!1
    let client = reqwest::blocking::Client::new();
    let users = client
        .get("https://slack.com/api/users.list") // reqwest::RequestBuilder
        .bearer_auth(auth_token) // reqwest::RequestBuilder
        .send()? // reqwest::blocking::response::Response
        .json::<UsersList>()? // UsersList
        ;

    // Yikes there are about 857 users
    let mut uid_username_mapping = HashMap::with_capacity(860);

    /* Let's just see what a randomly chosen user looks like (and by randomly chosen I mean I
    randomly picked the number 70; I'm not bringing in another dependency (specifically the rand
    crate) just for debugging purposes. See https://xkcd.com/221/) */
    if cfg!(debug_assertions) {
        println!("{:#?}", users.members[70]);
    }
    /* Extract the information we need from each member (If I ever decide to just go with storing
    queue::Users directly in the queue, (which I don't think can happen, see documentation for
    UserID) couldn't I just do something like
         users.members.iter().collect::<HashSet<_>>()
    ?) */
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
            .and_then(|prof| prof.real_name.clone());

        let username = user.profile.and_then(|prof| prof.display_name);

        let _ = uid_username_mapping.insert(UserID(id), (real_name, username));
    }

    Ok(uid_username_mapping)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_user_id() {
        let user_a = UserID::new("UA8RXUPSP");
        let user_b = UserID(String::from("UA8RXUPSP"));

        assert_eq!(user_a, user_b);
    }

    #[test]
    fn display() {
        let user = UserID(String::from("UA8RXUPSP"));

        assert_eq!(format!("{}", user), "UA8RXUPSP");
    }
}
