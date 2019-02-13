extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate humansize;
extern crate humantime;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate telegram_bot;
extern crate tokio_core;
extern crate tokio_signal;
extern crate urlencoding;

mod app;
mod cmd;
mod executor;
mod models;
mod msg;
mod schema;
mod state;
mod stats;
mod util;

use std::time::Duration;

use dotenv::dotenv;
use futures::{
    future::{ok, result},
    Future, Stream,
};
use telegram_bot::types::UpdateKind;
use tokio_core::reactor::{Core, Handle, Interval};
use tokio_signal::ctrl_c;

use msg::handler::Handler;
use state::State;
use util::handle_msg_error;

/// The application entrypoint.
// TODO: propegate errors upto this function
fn main() {
    // Load the environment variables file
    dotenv().ok();

    // Build a future reactor
    let mut core = Core::new().unwrap();

    // Initialize the global state
    let state = State::init(core.handle());

    // Build a signal handling future to quit nicely
    let signal = ctrl_c()
        .flatten_stream()
        .into_future()
        .inspect(|_| eprintln!("Received CTRL+C signal, preparing to quit..."))
        .map(|_| ())
        .map_err(|_| ());

    // Build the application, attach signal handling
    let app = build_application(state.clone(), core.handle())
        .select(signal)
        .map_err(|(e, _)| e)
        .then(|r| {
            state.stats().flush(state.db());
            eprintln!("Flushed stats to database");
            eprintln!("Quitting...");
            result(r)
        });

    // Run the application future in the reactor
    core.run(app).unwrap();
}

/// Build the future for running the main application, which is the bot.
fn build_application(state: State, handle: Handle) -> impl Future<Item = (), Error = ()> {
    let stats_flusher = build_stats_flusher(state.clone(), handle.clone());
    let telegram = build_telegram_handler(state, handle);

    telegram.join(stats_flusher).map(|_| ())
}

/// Build a future for handling Telegram API updates.
fn build_telegram_handler(state: State, handle: Handle) -> impl Future<Item = (), Error = ()> {
    state
        .telegram_client()
        .stream()
        .for_each(move |update| {
            // Clone the state to get ownership
            let state = state.clone();

            // Process messages
            match update.kind {
                UpdateKind::Message(message) => {
                    // Update the message stats
                    state.stats().increase_message_stats(&message, 1, 0);

                    // Build the message handling future, handle any errors
                    let msg_handler = Handler::handle(&state, message.clone()).or_else(|err| {
                        handle_msg_error(state, message, err).or_else(|err| {
                            eprintln!(
                                "ERR: failed to handle error while handling message: {:?}",
                                err,
                            );
                            ok(())
                        })
                    });

                    // Spawn the message handler future on the reactor
                    handle.spawn(msg_handler);
                }
                UpdateKind::EditedMessage(message) => {
                    state.stats().increase_message_stats(&message, 0, 1)
                }
                _ => {}
            }

            ok(())
        }).map_err(|err| {
            eprintln!("ERR: Telegram API updates loop error, ignoring: {}", err);
            ()
        })
}

/// Build a future for handling Telegram API updates.
// TODO: make the interval time configurable
fn build_stats_flusher(state: State, handle: Handle) -> impl Future<Item = (), Error = ()> {
    Interval::new(Duration::from_secs(60), &handle)
        .expect("failed to build stats flushing interval future")
        .for_each(move |_| {
            state.stats().flush(state.db());
            Ok(())
        }).map_err(|_| ())
}
