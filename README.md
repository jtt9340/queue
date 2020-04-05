# queue
A Slack bot that keeps track of people waiting in line for an event and notifies next in line when it is their turn

## The Actix Branch
This branch is from a while ago when I was using [actix-web](https://actix.rs) to implement this Slack bot instead of
[the Slack crate](crates.io/crates/slack). I hit a wall where I couldn't figure out how to get mutable access to my 
application state, as Actix only implements shared *immutable* application state (see [the Actix documentation](https://actix.rs/docs/extractors/)
at the bottom, where it says "Application state extractor"). The documentation explains that this is becuase Actix, being a
concurrent-forward designed web framework, instantiates application state for every thread that the app runs on, and shared
mutable state is typically a no-no in (safe) Rust. Furthermore, the Actix documentation pointed to using tokio synchronization
pritives if you needed to mutate your application state like I did (in order to add and remove people from the queue), which
I might have been able to figure out, but, at this point, I was already having more luck going the Slack crate route.

So this branch contains an abandoned would-be implementation of this Slack bot. It does not compile, hence why it is not the
master branch. Feel free to pick up where I ~left off~ got stuck and figure out how to use Actix to implement what I
accomplsihed using the Slack crate!
