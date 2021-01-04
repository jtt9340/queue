# Setting up a Slack Bot to work with the Slack crate 
This Slack bot is made possible by the [Slack crate], a Rust library that makes interfacing with the Slack API within Rust easy(-ier).
Without the Slack crate, one would have to use one of the many HTTP libraries for Rust (or even make one! :scream:) to make HTTP requests to the
various endpoints that the Slack API offers, but this isn't as easy as using the pre-made Slack crate in my opinion. Unfortunately, this comes with the
trade-off that the Slack crate only works with Slack's "classic apps," which are deprecated. Therefore, people wanting a write a Slack bot in Rust will have to make
a decision: whether they want to make a Slack botusing an easier yet deprectaed method, in which case they should follow these instructions; or
whether they want to use one of Rust's HTTP libraries instead to manually make all of the API calls.

Here are the steps I followed for setting up a Slack bot to work with the Slack crate:

1. Join the Slack workspace that you want your bot to run in.
2. Go to "https://api.slack.com/rtm#classic" and click "Create a classic Slack app".
3. Enter a name for your Slack bot and ensure you create it in the workspace that you joined in Step 1.
4. On the Basic Information page (see sidebar at left), under "Add features and functionality", click "Bots".
5. Click "Add Legacy Bot User".
6. Enter a display name and default userame for your bot.
7. Click "add".
8. Go back to the Basic Information page, under "Features and Functionality", click "Event Subscriptions".
9. Turn the switch in the upper right on.
10. `cargo new project_name`.
11. Make your Cargo.toml look like this:
```toml
[package]
name = <whatever you named your project>
description = <however you want to describe this project>
version = "0.1.0" # Doesn't literally have to be 0.1.0 but there also isn't a reason for you to change this (right now)
authors = ["Your Name <youremail@website.com>"]

# See more keys and their definitions...

[features]
actix = [ "actix-web", "acxtix-rt", "serde" ]

[[bin]]
name = "verify"
path = "src/verify.rs"
required-features = [ "actix" ]

[dependencies]
# These versions are probably out of date but I know they work, so I'm hesitant to upgrade.
# You can see for yourself what the latest versions are and try and see if it works with the latest version at crates.io
serde = { version = "1.0.104", features = [ "derive" ], optional = true }
actix-web = { version = "2.0.0", optional = true }
actix-rt = { version = "1.0.0", optional = true }
```
12. Create a file called `verify.rs` in the `src` directory with the following contents:
```rust
//! A "script" used to validate the Slack Events API. 
//!
//! Run ngrok with: `ngrok http PORT` (see PORT below)

use std::{io, net::Ipv4Addr};

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;

/// Which port number the host is bound to
const PORT: u16 = 3152;

#[derive(Deserialize, Debug)]
struct Payload {
    token: String,
    challenge: String,
    r#type: String,
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    // Which public url to connect to
    let socket_addr = (Ipv4Addr::LOCALHOST, PORT);
    
	// Let's set up a web server!
    HttpServer::new(|| App::new().route("/slack/events", web::post().to(post_handler)))
        .bind(socket_addr)?
        .run()
        .await
}

async fn post_handler(web::Json(response): web::Json<Payload>) -> impl Responder {
    println!("{:?}", response);
    HttpResponse::Ok().body(response.challenge)
}
```
13. Install [ngrok]. On macOS with Homebrew you can `brew install --cask ngrok`.
14. `ngrok http 3152`.
15. `cargo run --bin verify --features=actix` in another terminal window. You will see lots of things compiling.
16. With both ngrok and the verify program running in the background, go to the terminal window with ngrok running.
	You will see something like "Forwarding https://\<some code\>.ngrok.io -> http://localhost:3152". Copy the
	https://\<come code\>.ngrok.io URL and go back to the Event Subscriptions web page in your browser.
	Paste the URL into the text field labelled "Request URL." Then add "/slack/events" to the end of this URL. So overall
	the URL in the "Request URL" text field should be "https://\<some code\>.ngrok.io/slack/events." You will see a green
	"verified" checkmark.
17. You can kill ngrok and your verify program.
18. Click on "Subscribe to bot events".
19. Click "Add Bot User Event".
20. Add "app_mention" and "message.channels".
21. Click "Save Changes" at the bottom of your screen.
22. Go back to the Basic Information Page, under "Install app to your workspace" click "Install App to Workspace".
23. Click "allow" .
24. To get your API token, on the Install App page (see sidebar at left) copy the string of letters and numbers under "Bot User OAuth Access Token".

[Slack crate]: https://crates.io/crates/slack 
[ngrok]: https://ngrok.com/
