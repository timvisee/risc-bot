use async_trait::async_trait;
use failure::{Error as FailureError, SyncFailure};
use futures::prelude::*;
use telegram_bot::{
    prelude::*,
    types::{Message, ParseMode},
    Error as TelegramError,
};

use super::Action;
use crate::app::{NAME, VERSION};
use crate::state::State;

/// The action command name.
const CMD: &str = "risc";

/// Whether the action is hidden.
const HIDDEN: bool = false;

/// The action help.
const HELP: &str = "RISC info";

pub struct Risc;

impl Risc {
    pub fn new() -> Self {
        Risc
    }
}

#[async_trait]
impl Action for Risc {
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
        // Build a future for sending the response message
        state
            .telegram_send(
                msg.text_reply(format!(
                    "\
                     `{} v{}`\n\
                     \n\
                     Developed by @timvisee\n\
                     https://timvisee.com/\n\
                     \n\
                     Source:\n\
                     https://gitlab.com/timvisee/risc-bot\
                     ",
                    NAME, VERSION,
                ))
                .parse_mode(ParseMode::Markdown),
            )
            .map_ok(|_| ())
            .map_err(|err| Error::Respond(SyncFailure::new(err)).into())
            .await
    }
}

/// A start action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
