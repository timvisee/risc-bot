use failure::{Error as FailureError, SyncFailure};
use futures::Future;
use telegram_bot::{
    prelude::*,
    types::{Message, ParseMode},
    Error as TelegramError,
};

use super::Action;
use state::State;

/// The action command name.
const CMD: &'static str = "test";

/// Whether the action is hidden.
const HIDDEN: bool = true;

/// The action help.
const HELP: &'static str = "Test command";

pub struct Test;

impl Test {
    pub fn new() -> Self {
        Test
    }
}

impl Action for Test {
    fn cmd(&self) -> &'static str {
        CMD
    }

    fn hidden(&self) -> bool {
        HIDDEN
    }

    fn help(&self) -> &'static str {
        HELP
    }

    fn invoke(&self, state: &State, msg: &Message) -> Box<Future<Item = (), Error = FailureError>> {
        // Build a future for sending the response message
        let future = state
            .telegram_send(
                msg.text_reply("<i>Jep... works on my machine!</i>")
                    .parse_mode(ParseMode::Html),
            )
            .map(|_| ())
            .map_err(|err| Error::Respond(SyncFailure::new(err)))
            .from_err();

        Box::new(future)
    }
}

/// A test action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
