use async_trait::async_trait;
use failure::{Error as FailureError, SyncFailure};
use futures::prelude::*;
use telegram_bot::{
    prelude::*,
    types::{Message, MessageKind, ParseMode},
    Error as TelegramError,
};

use super::Action;
use crate::state::State;

/// The action command name.
const CMD: &str = "echo";

/// Whether the action is hidden.
const HIDDEN: bool = true;

/// The action help.
const HELP: &str = "Echo user input as Markdown";

pub struct Echo;

impl Echo {
    pub fn new() -> Self {
        Echo
    }
}

#[async_trait]
impl Action for Echo {
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
        if let MessageKind::Text { ref data, .. } = &msg.kind {
            // Get the user's input
            // TODO: actually properly fetch the user input
            let input = data
                .splitn(2, ' ')
                .nth(1)
                .map(|cmd| cmd.trim_start())
                .unwrap_or("")
                .to_owned();

            // Build a future for sending the response message
            state
                .telegram_send(msg.text_reply(input).parse_mode(ParseMode::Markdown))
                .map_ok(|_| ())
                .map_err(|err| Error::Respond(SyncFailure::new(err)).into())
                .await
        } else {
            Ok(())
        }
    }
}

/// A echo action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
