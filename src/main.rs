#[macro_use]
extern crate diesel;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;

mod app;
mod cmd;
mod executor;
mod models;
mod msg;
mod schema;
mod state;
mod stats;
pub mod traits;
mod util;

use std::time::Duration;

use dotenv::dotenv;
use futures::future;
use futures::prelude::*;
use telegram_bot::types::UpdateKind;
use tokio::pin;
use tokio::runtime::Handle;
use tokio::signal::ctrl_c;
use tokio_stream::wrappers::IntervalStream;

use msg::handler::Handler;
use state::State;
use util::handle_msg_error;

/// Maximum number of updates handled concurrently.
const MAX_CONCURRENT_UPDATES: usize = 4;

/// The application entrypoint.
#[tokio::main]
async fn main() {
    // Load the environment variables file
    dotenv().ok();

    // Initialize the global state
    let state = State::init(Handle::current());

    // Build a signal handling future to quit nicely
    let signal = ctrl_c().inspect(|_| eprintln!("Received CTRL+C signal, preparing to quit..."));
    pin!(signal);

    // Build the application, attach signal handling
    let app = build_application(state.clone(), Handle::current());
    let app = future::select(app, signal).then(|_| {
        state.stats().flush(state.db());
        eprintln!("Flushed stats to database");
        eprintln!("Quitting...");
        future::ready(())
    });

    // Run the application future in the reactor
    app.await
}

/// Build the future for running the main application, which is the bot.
fn build_application(state: State, handle: Handle) -> impl Future<Output = ()> + Unpin {
    let stats_flusher = build_stats_flusher(state.clone());
    let telegram = build_telegram_handler(state, handle);
    future::select(telegram, stats_flusher).map(|_| ())
}

/// Build a future for handling Telegram API updates.
fn build_telegram_handler(state: State, handle: Handle) -> impl Future<Output = ()> {
    state.telegram_client().stream().for_each_concurrent(
        self::MAX_CONCURRENT_UPDATES,
        move |update| {
            // Clone the state to get ownership
            let state = state.clone();

            // Unpack update
            // TODO: return errors?
            let update = match update {
                Ok(update) => update,
                Err(err) => {
                    eprintln!("ERR: Telegram API updates loop error, ignoring: {}", err);
                    return future::ready(());
                }
            };

            // Process messages
            match update.kind {
                UpdateKind::Message(message) => {
                    // Update the message stats
                    state.stats().increase_message_stats(&message, 1, 0);

                    // Build the message handling future, handle any errors
                    let msg_handler =
                        Handler::handle(state.clone(), message.clone()).or_else(|err| {
                            handle_msg_error(state, message, err).map_err(|err| {
                                eprintln!(
                                    "ERR: failed to handle error while handling message: {:?}",
                                    err,
                                );
                            })
                        });

                    // Spawn the message handler future on the runtime
                    handle.spawn(msg_handler);
                }
                UpdateKind::EditedMessage(message) => {
                    state.stats().increase_message_stats(&message, 0, 1);
                }
                _ => {}
            }

            future::ready(())
        },
    )
}

/// Build a future for handling Telegram API updates.
///
/// Returned future never completes.
fn build_stats_flusher(state: State) -> impl Future<Output = ()> {
    let interval = tokio::time::interval(Duration::from_secs(60));
    IntervalStream::new(interval).for_each(move |_| {
        state.stats().flush(state.db());
        future::ready(())
    })
}
