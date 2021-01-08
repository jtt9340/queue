use std::{env, process};

use getopts::Options;

pub use print_queue::{queue, user, CHANNEL};
use user::create_uid_username_mapping;

/// Display usage information. Used for handling the "-h" or "--help" flags if passed, or if the Slack
/// API key was not given, as that is a _required_ command line argument.
fn usage(program: &str, opts: Options) {
    let desc = format!(
        "Queue \u{2014} a Slack bot to keep track of who is using a 3D \
	printer\nUsage:\n\t{} (-k api-KEY | --key API-KEY>) [-f FILE | --file FILE] [-h | --help]",
        program
    );
    print!("{}", opts.usage(&desc));
}

/// Entry point for the Slack bot.
fn main() -> Result<(), slack::error::Error> {
    let mut args = env::args();
    let program = args
        .next()
        .expect("Program name was not passed to command line arguments");

    let mut opts = Options::new();
    opts.optopt("k", "key", "Slack bot API key", "API-KEY");
    opts.optopt(
        "f",
        "file",
        "name of the backup file to use; will be created if empty",
        "FILE",
    );
    opts.optopt(
        "c",
        "channel",
        "The name of the channel to run the bot in",
        "CHANNEL-NAME",
    );
    opts.optflag("h", "", "show a one-line usage summary");
    opts.optflag("", "help", "display this help message and exit");

    let matches = match opts.parse(args) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("{}", f.to_string());
            usage(&program, opts);
            process::exit(-3);
        }
    };

    // Exit immediately if -h or --help is passed
    if matches.opt_present("h") {
        println!("{}", opts.short_usage(&program));
        return Ok(());
    } else if matches.opt_present("help") {
        usage(&program, opts);
        return Ok(());
    }

    let api_key = match matches.opt_str("key") {
        Some(api_key) => api_key,
        None => {
            eprintln!("Required option \'key\' missing");
            usage(&program, opts);
            process::exit(-1);
        }
    };

    let users = match create_uid_username_mapping(api_key.as_str()) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(-2);
        }
    };

    // Only run the following in debug (i.e. not release) mode
    if cfg!(debug_assertions) {
        println!("{:#?}", users);
        println!("Number of members: {:?}", users.len());
    }

    let channel = matches.opt_str("channel").unwrap_or(String::from(CHANNEL));

    let mut queue = match matches.opt_str("f") {
        Some(file) => queue::Queue::from_file(&*channel, &users, file),
        None => queue::Queue::new(&*channel, &users),
    };
    slack::RtmClient::login_and_run(&api_key, &mut queue)
}
