use async_trait::async_trait;
use failure::{Error as FailureError, SyncFailure};
use futures::prelude::*;
use telegram_bot::{prelude::*, types::Message, Error as TelegramError};

use super::Action;
use crate::state::State;

/// The action command name.
const CMD: &str = "ping";

/// Whether the action is hidden.
const HIDDEN: bool = false;

/// The action help.
const HELP: &str = "Ping RISC";

pub struct Ping;

impl Ping {
    pub fn new() -> Self {
        Ping
    }
}

#[async_trait]
impl Action for Ping {
    fn cmd(&self) -> &'static str {
        CMD
    }

    fn hidden(&self) -> bool {
        HIDDEN
    }

    fn help(&self) -> &'static str {
        HELP
    }

    async fn invoke(&self, state: State, msg: Message) -> Result<(), FailureError> {
        // Build a message future for sending the response
        state
            .telegram_send(msg.text_reply("Pong!"))
            .map_ok(|_| ())
            .map_err(|err| Error::Respond(SyncFailure::new(err)).into())
            .await
    }
}

/// A ping action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
