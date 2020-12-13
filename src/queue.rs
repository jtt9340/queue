use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    fs::{File, OpenOptions},
    io::{self, BufWriter, Seek, SeekFrom},
    ops::Deref,
    path::Path,
};

use slack::RtmClient;

use crate::queue::{AddResult::*, RemoveResult::*};
use crate::user::{SlackMap, UserID};

/// The User ID (a string of the form UXXXXXXX) for the Queue app
pub const QUEUE_UID: &str = "<@U01A844Q2US>";

/// Sometimes we need these.
pub const INSPIRATIONAL_QUOTE: &str =
	"_Waiting in line is a great opportunity to meet people, daydream, or play._\n\t\u{2014}Patch Adams";

/// Which Slack channel Queue is running in.
const CHANNEL: &str = "3d-printer-queue";

/// A help message to display when the `help` command is invoked
const USAGE: &str = "*Queue* is a :slack: bot that keeps track of who is waiting in line to use the \
3D printers. You interact with it by @mentioning it and then typing a command (e.g. `@Queue help`).\
Here are the different commands *Queue* currently recognizes:\n\n\
• *add*: Add yourself to the queue. You can add yourself multiples times, in case there are multiple \
things you want to 3D print. However, you cannot have two back-to-back instances of yourself in the \
queue so that you let others get a chance. However, if the queue is relatively empty (and by relatively \
empty I mean less than 3 people in line), then you _can_ have back-to-back instances of yourself, since \
not as many people are being negatively affected by having back-to-back instances of yourself in the queue \
as they would be if there were more than 3 people in line.\n\
• *done*: Leave the queue. If there are multiple instances of you in the queue, the _first_ instance \
(i.e. the one closest to the front) is removed. If you were in 0th place when you were removed, the \
person is 1st place is notified of this change.\n\
• *show*: See who is in the queue and in what place.\n\
• *help*: Display this message.";

/// Given the body of a post to Slack, determine someone mentioned the Queue app
fn is_app_mention(text: &str) -> bool {
    text.contains(QUEUE_UID)
}

/// The main data structure for keeping track of Slack users for an event.
#[derive(Debug)]
pub struct Queue<'a> {
    /// A queue of references to UserIDs in the `uid_username_mapping`
    queue: VecDeque<UserID>,
    /// All the possible members of a Slack workspace that can join a queue
    uid_username_mapping: &'a SlackMap,
    /// The file that `self` will write to to preserve its state (may be a database connection in the future)
    db_conn: BufWriter<File>,
}

/// A type used to represent the result of adding a user to the queue.
#[derive(Debug)]
pub enum AddResult {
    /// The specified user was *not* added to the queue because they need to let someone else take a
    /// turn before they add themselves again.
    UserNotAdded,
    /// The specified user was successfully added.
    UserSuccessfullyAdded,
    /// The specified user was added to the queue, but there was an I/O error while writing to a file
    /// that keeps the queue persistent, so the backup file and the true state of the queue are now
    /// out of sync.
    UserUnsuccessfullyAdded(io::Error),
}

/// A type used to represent the result a removing a user from the queue.
#[derive(Debug)]
pub enum RemoveResult {
    /// The user was not the the queue in the first place, so they cannot be removed!
    NonExistentUser,
    /// The user was successfully removed from the queue. This variant contains the position that
    /// they were in before they were removed from the queue.
    UserSuccessfullyRemoved(usize),
    /// The specified user was removed from the queue, but there was an I/O error while writing to a
    /// file that keeps the queue persistent, so the backup file and the true state of the queue are
    /// now out of sync.
    UserUnsuccessfullyRemoved(io::Error),
}

impl<'a> Queue<'a> {
    /// Create an empty queue with no previous state. `uids_to_users` is a `std::collections::HashMap`
    /// whose keys are Slack IDs and whose values are usernames associated with the given Slack ID.
    ///
    /// This function will also create an empty file that, over the course of the lifetime of this
    /// queue, will be written to representing the users in the queue so that, if the app were to
    /// crash, the queue isn't lost. By default, this file's name is `queue_state.txt`. If you want
    /// to use a different name, use [`Queue::from_file`](#method.from_file)
    ///
    /// # Panics
    /// This function will panic if the aforementioned `queue_state.txt` fails to be created.
    pub fn new(uids_to_users: &'a SlackMap) -> Self {
        Self {
            queue: VecDeque::new(),
            uid_username_mapping: uids_to_users,
            db_conn: BufWriter::new(
                File::create("queue_state.txt")
                    .expect("Could not create a backup file for the queue"),
            ),
        }
    }

    /// Create a queue whose state is described by the file located at `path`, effectively restoring
    /// it from a previous state. The file should be one that was previously created by running this
    /// app. Or, if the file does not exist, it will be created.
    ///
    /// # Examples
    /// Say the input file contains the following content.
    /// ```text
    /// 0 UA8RXUPSP
    /// 1 UNB2LMZRP
    /// 2 UN480W9ND
    /// ```
    /// Then invoking this function with the above file will create a new `Queue` where the first
    /// person in line has an ID of `UA8RXUPSP`, the second person in line has an ID of `UNB2LMZRP`,
    /// and the third person in line has an ID of `UN480W9ND`.
    ///
    /// # Panics
    /// For better or worse, there are many ways this function can panic.
    /// * If the file breaks the "rules" (see the documentation for
    /// [`Queue::add_user`](#method.add_user)) of adding people to the queue (e.g. the file contains
    /// the same person five times in a row, which shouldn't be allowed under any circumstance), then
    /// the file will be rejected and this function will panic.
    /// * If the file at `path` fails to open for any reason (e.g. permissions).
    /// * If the file at `path` fails to be read for any reason.
    /// * If the file is _not_ in the valid format expected by queue: each line is a positive integer,
    /// followed by a tab, followed by a Slack user ID.
    pub fn from_file<P: AsRef<Path>>(uids_to_users: &'a SlackMap, path: P) -> Self {
        use std::io::Read; // needed for the invocation of read_to_string()

        let mut people = BTreeMap::new();
        let mut backup_file_contents = String::new();
        let backup_file_name = path.as_ref().as_os_str().to_owned();
        let mut backup_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .expect("Could not open backup file for the queue");

        if let Err(e) = backup_file.read_to_string(&mut backup_file_contents) {
            panic!("could not read {:?}: {}", backup_file_name, e);
        }

        // Iterate only over the lines that have content (i.e. are not all whitespace)
        for line in backup_file_contents
            .lines()
            .filter(|s| !s.trim().is_empty())
        {
            let [pos, uid] = {
                let err_msg = "Invalid file format: each line must contain a parse-able \
				positive integer followed by some amount of whitespace, followed by a Slack user-id";
                let mut iter = line.split_whitespace();
                [iter.next().expect(err_msg), iter.next().expect(err_msg)]
            };
            let pos = pos.parse::<usize>().expect(
                "Invalid file format: each line in the file must start with a parse-\
				able positive integer",
            );
            if let Some(_) = people.insert(pos, uid) {
                panic!("Invalid file format: only one person per position (index) in line");
            }
        }

        let mut queue = Self {
            queue: VecDeque::with_capacity(people.len()),
            uid_username_mapping: uids_to_users,
            db_conn: BufWriter::new(backup_file),
        };

        for (pos, person) in people {
            if !queue.add_user_no_write(UserID::new(person)) {
                panic!(
                    "user {} in position {} \"breaks the addition rules\": see the Queue document\
				ation for more",
                    person, pos
                );
            }
        }

        queue
    }

    /// Writes the current state of `self` to `self.db_conn` so that this particular state can be
    /// reloaded later.
    fn write_state(&mut self) -> io::Result<()> {
        use std::io::Write; // needed for the invocation of std::io::Write::flush

        // We write the state to a temporary buffer before writing the entirety of the buffer to the
        // file.
        let mut output = Vec::new();
        // For each user in the queue, write the line
        // {user position}<tab>{user ID}
        // returning early if any line fails.
        // Otherwise, flush the BufWriter to the file and hope it works :P
        for (pos, uid) in self.queue.iter().enumerate() {
            write!(output, "{}\t{}\n", pos, uid)?;
        }

        // Get the number of bytes in the file currently
        let num_bytes = self.db_conn.seek(SeekFrom::End(0))?;
        // Now go back to the start of the file
        self.db_conn.seek(SeekFrom::Start(0))?;
        // And now create a bunch of blanks to erase the file
        let blanks = vec![b' '; num_bytes as usize];
        self.db_conn.write_all(&*blanks)?;
        // This is getting tiring...go back to the start of the file
        self.db_conn.seek(SeekFrom::Start(0))?;
        // Write the new state
        self.db_conn.write_all(output.as_slice())?;
        self.db_conn.flush()
    }

    /// Can `user` be added to `self` based on the following rules?
    ///
    /// 1. If the queue is *not empty*, then a user can only be added to the queue if the person in
    /// front of them is __not themselves__.
    /// 2. If the queue *is empty*, then a user can be added up to three times.
    fn can_add(&self, user: &UserID) -> bool {
        self.len() < 3 || self.back() != Some(user)
    }

    /// Add a user to the back of the queue _without_ writing to the backup file, returning true if
    /// the user could be added per the rules, and false otherwise. See
    /// [`Queue::add_user`](#method.add_user) for more.
    fn add_user_no_write(&mut self, user: UserID) -> bool {
        if self.can_add(&user) {
            self.queue.push_back(user);
            true
        } else {
            false
        }
    }

    /// Add a User to the back of the queue.
    ///
    /// People are allowed to be in the queue multiple times. The rules are as follows:
    /// 1. If the queue is *not empty*, then a user can only be added to the queue if the person in front
    /// them is __not themselves__.
    /// 2. If the queue *is empty*, then a user can be added up to three times.
    ///
    /// This function will write to the backup file that persists the state of the queue. If that
    /// write fails, then an `(u, AddResult::UserUnsuccessfullyAdded(e))` is returned, where `u` is
    /// *a reference to* the user that was just added to the queue but __not__ to the backup file,
    /// and `e` is a `std::io::Error` describing what went wrong. If the user couldn't be added to the
    /// queue in the first place (because the addition would have violated the rules stated above),
    /// then __no file I/O occurs__ and a `(u, AddResult::UserNotAddded)` is returned, where `u` is
    /// *a reference to* the user that was *going to be* added. Otherwise, a
    /// `(u, AddResult::UserSuccessfullyAdded)` is returned, where `u` is a *a reference to* the user
    /// that was just added to the queue.
    pub fn add_user(&mut self, user: UserID) -> (UserID, AddResult) {
        if self.add_user_no_write(user.clone()) {
            match self.write_state() {
                Ok(()) => (user, UserSuccessfullyAdded),
                Err(e) => (user, UserUnsuccessfullyAdded(e)),
            }
        } else {
            (user, UserNotAdded)
        }
    }

    /// Handle the add command. Returns a message to post in the Slack channel depending on whether
    /// or not the user was actually added.
    fn add(&mut self, user: UserID) -> String {
        match self.add_user(user) {
            (user, UserSuccessfullyAdded) => {
                format!("Okay <@{}>, I have added you to the queue.", user)
            }
            (user, UserNotAdded) => format!(
                "<@{}>, you cannot be added to the queue at \
			this time. Please let others get a chance to wait in line before you go again.",
                user
            ),
            (user, UserUnsuccessfullyAdded(e)) => format!(
                "Hi <@{}>. You have been \
			added to the queue, but this change has not been reflected in the backup file that stores \
			the state of the queue. If it helps, the reason why is: {}",
                user, e
            ),
        }
    }

    /// Handle the done command. Returns a message to post in the Slack channel depending on whether
    /// or not the user was removed.
    fn done(&mut self, user: UserID) -> String {
        match self.remove_user(user) {
            (user, UserSuccessfullyRemoved(idx)) => {
                let mut response = format!(
                    "Okay <@{}>, you have been removed from{}the queue.",
                    user.0,
                    if idx == 0 { " the front of " } else { " " }
                );
                // If the person just removed was at the front, then notify the next person in line
                // (if there is one)
                if idx == 0 {
                    match self.peek_first_user_in_line() {
                        Some(next) => {
                            response.push_str("\nHey <@");
                            response.push_str(&*next.0);
                            response.push_str(">! You\'re next in line!");
                        }
                        None => response.push_str("\nNobody is next in line!"),
                    }
                }
                response
            }
            (user, NonExistentUser) => format!(
                "<@{}>, you cannot be removed; you are not \
			in the queue.",
                user
            ),
            (user, UserUnsuccessfullyRemoved(e)) => format!(
                "Hi <@{}>. You were removed \
			from the queue, but this change has not been reflected in the backup file that stores \
			the state of the queue. If it helps, the reason why is: {}",
                user, e
            ),
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
    /// Since the Queue now can hold multiple instances of the same person, this will remove the _first_
    /// instance of the person. For example, say you are in the third, sixth, and eighth positions in
    /// the queue. If you elect to remove yourself from the queue, you will still be in the sixth
    /// and eighth positions in the queue, but you will no longer be in the third position.
    /// TODO: add a method for removing _all_ instances of oneself in the queue
    ///
    /// This function will write to the backup file that persists the state of the queue. If that write
    /// fails, then `Err(e)` is returned, where `e` is an error object describing the error. Otherwise,
    /// if the given user was able to be removed (i.e. at least one instance of them was in the queue),
    /// then an `Ok(Some((u, i))` is returned, where `u` is the user removed and `i` is the number
    /// position `u` was in _before_ they were removed from the queue (0 is the first position in the
    /// queue). In all other cases, `Ok(None)` is returned.
    pub fn remove_user(&mut self, user: UserID) -> (UserID, RemoveResult) {
        match self.queue.iter().position(|u| *u == user) {
            Some(idx) => {
                // If we attempt to remove a non-existent user, Iter::position will return None, so
                // *in theory* idx should refer to a valid index in the queue.
                let removed = self
                    .queue
                    .remove(idx)
                    .expect("Attempted to remove a non-existent user");
                match self.write_state() {
                    Ok(()) => (removed, UserSuccessfullyRemoved(idx)),
                    Err(e) => (removed, UserUnsuccessfullyRemoved(e)),
                }
            }
            None => (user, NonExistentUser),
        }
    }

    /// Given a Slack ID, return the real name-maybe username pair associated with that ID, if there is one.
    fn get_username_by_id(&self, id: &UserID) -> Option<&(Option<String>, Option<String>)> {
        self.uid_username_mapping.get(id)
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
        let lowercase_queue_id = QUEUE_UID.to_lowercase();
        let body = body.to_lowercase();
        let body = body.trim_start_matches(lowercase_queue_id.as_str());

        match body.trim() {
            "add" => self.add(user),
            // "cancel" => self.cancel(user),
            "done" => self.done(user),
            "show" => format!("{}", self),
            "help" => String::from(USAGE),
            s => format!("Unrecognized command {}. Try `@Queue help`.", s),
        }
    }
}

impl slack::EventHandler for Queue<'_> {
    fn on_event(&mut self, cli: &RtmClient, event: slack::Event) {
        if cfg!(debug_assertions) {
            println!("Got event: {:?}", event);
        }
        match event {
            slack::Event::Message(message) => {
                if let slack::Message::Standard(ms) = *message {
                    // The content of the message
                    let text = ms.text.unwrap_or_default();
                    if is_app_mention(&*text) {
                        // Who posted the message
                        let user = ms.user.expect("User does not exist");
                        // What to send back to Slack
                        let response = self.determine_response(UserID(user), text.as_str());
                        // The channel the message was posted in
                        let chan = ms.channel.expect("Channel does not exist");
                        // Send 'em back!
                        let _ = cli.sender().send_message(&*chan, &*response);
                    }
                }
            }
            _ => (),
        };
    }

    fn on_close(&mut self, _cli: &RtmClient) {
        println!("on_close");
    }

    fn on_connect(&mut self, cli: &RtmClient) {
        println!("{}", INSPIRATIONAL_QUOTE);
        let chan_id = cli
            .start_response()
            .channels
            .as_ref()
            .and_then(|chans| {
                chans.iter().find(|chan| match chan.name {
                    None => false,
                    Some(ref name) => name == CHANNEL,
                })
            })
            .and_then(|chan| chan.id.as_ref())
            .expect("channel botspam not found");
        let _ = cli.sender().send_message(&chan_id, INSPIRATIONAL_QUOTE);
    }
}

impl fmt::Display for Queue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Here are the people currently in line:\n{}",
            self.queue
                .iter()
                .enumerate()
                .map(|idx_user_pair| {
                    let (idx, u) = idx_user_pair;
                    let (maybe_real_name, maybe_username) = self
                        .get_username_by_id(u)
                        .expect(format!("For some reason user {} did not have an ID", u).as_str());
                    let u = &u.to_string();
                    let real_name = maybe_real_name.as_ref().unwrap_or(u);
                    match maybe_username {
                        Some(uname) if !uname.is_empty() => {
                            format!("{}. {} ({})\n", idx, real_name, uname)
                        }
                        _ => format!("{}. {}\n", idx, real_name),
                    }
                })
                .fold(String::default(), |acc, line| acc.to_owned() + &line)
        )
    }
}

impl Deref for Queue<'_> {
    type Target = VecDeque<UserID>;

    fn deref(&self) -> &Self::Target {
        &self.queue
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn create_queue() -> Result<(), String> {
        let hash_map = HashMap::new();
        let test_file = match File::create("queue_state_2.txt") {
            Ok(f) => f,
            Err(e) => return Err(format!("{}", e)),
        };

        let queue_a = Queue::new(&hash_map);
        let queue_b = Queue {
            queue: VecDeque::new(),
            uid_username_mapping: &hash_map,
            db_conn: BufWriter::new(test_file),
        };

        if !queue_a.is_empty() {
            return Err(String::from("Queue::new does not return an empty queue"));
        }

        if queue_a.queue != queue_b.queue {
            return Err(format!(
                "queue_a.queue ({:?}) != queue_b.queue ({:?})",
                queue_a.queue, queue_b.queue
            ));
        }

        if queue_a.uid_username_mapping == queue_b.uid_username_mapping {
            Ok(())
        } else {
            Err(format!(
                "queue_a.uid_username_mapping ({:?}) != queue_b.uid_username_mapping ({:?})",
                queue_a.uid_username_mapping, queue_b.uid_username_mapping
            ))
        }
    }

    fn add_users_helper(queue: &mut Queue, user: UserID) {
        let (new_user, result) = queue.add_user(user.clone());
        assert_eq!(
            new_user, user,
            "Queue::add_user does not return just-added user"
        );
        match result {
            UserSuccessfullyAdded => (), // This is the intended behavior
            res => panic!(
                "{} was not added to the queue properly: {:?}",
                new_user, res
            ),
        }
    }

    #[test]
    fn add_users() {
        let hash_map = HashMap::new();
        let mut queue = Queue::new(&hash_map);

        add_users_helper(&mut queue, UserID::new("UA8RXUPSP"));
        add_users_helper(&mut queue, UserID::new("UNB2LMZRP"));
        add_users_helper(&mut queue, UserID::new("UN480W9ND"));

        assert_eq!(
            queue.queue,
            [
                UserID::new("UA8RXUPSP"),
                UserID::new("UNB2LMZRP"),
                UserID::new("UN480W9ND"),
            ]
        );
    }

    /// This function tests Rule 1 of adding users to the queue: a user can only be added to the queue
    /// if the last person in the queue is not themselves. This gives other people a chance to wait
    /// in line.
    #[test]
    fn add_duplicate_users_to_nonempty_queue() {
        let hash_map = HashMap::new();
        let mut queue = Queue::new(&hash_map);
        let test_user = UserID::new("UA8RXUPSP");

        add_users_helper(&mut queue, test_user.clone());
        add_users_helper(&mut queue, UserID::new("UNB2LMZRP"));
        // Rule 1 states that a user can be added to the queue iff the person that would be in front
        // of them is not themselves, so this addition should work
        add_users_helper(&mut queue, test_user.clone());

        // However, they are now the last person in the queue so if they try to add themselves again
        // it shouldn't work
        assert!(!queue.can_add(&test_user));

        let (new_user, result) = queue.add_user(test_user);
        match result {
            UserNotAdded => (), // This is the intended behavior
            res => panic!("{} was erroneously added to the queue: {:?}", new_user, res),
        }

        assert_eq!(
            queue.queue,
            [
                UserID::new("UA8RXUPSP"),
                UserID::new("UNB2LMZRP"),
                UserID::new("UA8RXUPSP"),
            ]
        );
    }

    /// This function tests Rule 2 of adding users to the queue. A user is allowed to add themselves
    /// up to three times to the queue if it is initially empty.
    #[test]
    fn add_duplicate_users_to_empty_queue() {
        let hash_map = HashMap::new();
        let mut queue = Queue::new(&hash_map);

        // This should work, because the queue is empty so UA8RXUPSP can add themselves up to 3 times
        for _ in 0..3 {
            let test_user = UserID::new("UA8RXUPSP");
            assert!(queue.can_add(&test_user));
            add_users_helper(&mut queue, test_user);
        }

        // But this should fail, because now it's time to let someone else have a turn
        let test_user = UserID::new("UA8RXUPSP");
        assert!(!queue.can_add(&test_user));
        let (new_user, result) = queue.add_user(test_user);
        match result {
            UserNotAdded => (), // This is the intended behavior
            res => panic!(
                "{} was added to an empty queue for the fourth time: {:?}",
                new_user, res
            ),
        }
    }

    #[test]
    fn remove_front_users() {
        let hash_map = HashMap::new();
        let mut queue = Queue::new(&hash_map);

        add_users_helper(&mut queue, UserID::new("UA8RXUPSP"));
        add_users_helper(&mut queue, UserID::new("UNB2LMZRP"));
        add_users_helper(&mut queue, UserID::new("UN480W9ND"));

        let test_user = UserID::new("UA8RXUPSP");

        match queue.remove_user(UserID::new("UA8RXUPSP")) {
            (u, UserSuccessfullyRemoved(0)) if u == test_user => (), // This is the expected behavior
            res => panic!("Queue::remove_user returned unexpected result: {:?}", res),
        }

        assert_eq!(
            queue.queue,
            [UserID::new("UNB2LMZRP"), UserID::new("UN480W9ND"),]
        );

        // Empty the queue
        if let (_, UserUnsuccessfullyRemoved(ioe)) = queue.remove_user(UserID::new("UNB2LMZRP")) {
            panic!("{}", ioe);
        }
        if let (_, UserUnsuccessfullyRemoved(ioe)) = queue.remove_user(UserID::new("UN480W9ND")) {
            panic!("{}", ioe);
        }

        assert!(queue.queue.is_empty());
    }

    #[test]
    fn peek_front_users() {
        let hash_map = HashMap::new();
        let mut queue = Queue::new(&hash_map);

        add_users_helper(&mut queue, UserID::new("UA8RXUPSP"));
        add_users_helper(&mut queue, UserID::new("UNB2LMZRP"));
        add_users_helper(&mut queue, UserID::new("UN480W9ND"));

        assert_eq!(
            queue.peek_first_user_in_line(),
            Some(&UserID::new("UA8RXUPSP")),
            "Queue::peek_first_user_in_line does not return a reference to the front user"
        );

        // Does not mutate the queue itself
        assert_eq!(
            queue.queue,
            [
                UserID::new("UA8RXUPSP"),
                UserID::new("UNB2LMZRP"),
                UserID::new("UN480W9ND"),
            ],
            "Queue::peek_first_user_in_line mutates the queue when it is not supposed to"
        );
    }

    #[test]
    fn remove_arbitrary_users() {
        let hash_map = HashMap::new();
        let mut queue = Queue::new(&hash_map);

        add_users_helper(&mut queue, UserID::new("UA8RXUPSP"));
        add_users_helper(&mut queue, UserID::new("UNB2LMZRP"));
        add_users_helper(&mut queue, UserID::new("UN480W9ND"));

        let test_user = UserID::new("UNB2LMZRP");

        match queue.remove_user(UserID::new("UNB2LMZRP")) {
            (u, UserSuccessfullyRemoved(1)) if u == test_user => (), // This is the expected behavior
            res => panic!(
                "Queue::remove_user returned an unexpected result: {:?}",
                res
            ),
        }

        assert_eq!(
            queue.queue,
            [UserID::new("UA8RXUPSP"), UserID::new("UN480W9ND"),]
        );
    }

    #[test]
    fn remove_non_existent_users() {
        let hash_map = HashMap::new();
        let mut queue = Queue::new(&hash_map);

        match queue.remove_user(UserID::new("UNB2LMZRP")) {
            (_, NonExistentUser) => (), // This is the behavior that is expected
            res => panic!(
                "Queue::remove_user erroneously returns a user when \
			trying to remove a user from an empty queue. The result returned was: {:?}",
                res
            ),
        }

        add_users_helper(&mut queue, UserID::new("UNB2LMZRP"));
        add_users_helper(&mut queue, UserID::new("UN480W9ND"));

        match queue.remove_user(UserID::new("UA8RXUPSP")) {
            (_, NonExistentUser) => (), // This is the behavior that is expected
            res => panic!(
                "Queue::remove_user erroneously removes a user not \
			in the queue. The user returned was: {:?}",
                res
            ),
        }
    }
}
