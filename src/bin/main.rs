use std::{env, process};

use getopts::Options;

pub use print_queue::queue;
pub use print_queue::user;
use user::create_uid_username_mapping;

/// Entry point for the Slack bot.
fn main() -> Result<(), slack::error::Error> {
	let args = env::args();

	let mut opts = Options::new();
	opts.reqopt(
		"k",
		"key",
		"Slack bot API key",
		"API-KEY",
	);
	opts.optopt(
		"f",
		"file",
		"name of the backup to use; will be created if empty",
		"FILE",
	);
	opts.optflag("h", "help", "show this help menu and exit");

	let matches = match opts.parse(args) {
		Ok(m) => m,
		Err(f) => panic!(f.to_string()),
	};

	let api_key = match args.len() {
		0 | 1 => {
			eprintln!("No API key in args! Usage: cargo run --bin slack_main --features slack -- <api-key>");
			process::exit(-1);
		},
		nargs => args[nargs - 1].clone(),
	};

	let users = match create_uid_username_mapping(api_key.as_str()) {
		Ok(u) => u,
		Err(e) => {
			eprintln!("{}", e);
			process::exit(-2);
		},
	};
	println!("{:#?}", users);
	println!("Number of members: {:?}", users.len());

	let mut queue = queue::Queue::new(&users);
	slack::RtmClient::login_and_run(&api_key, &mut queue)
}