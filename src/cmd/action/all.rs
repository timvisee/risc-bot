use async_trait::async_trait;
use failure::{Error as FailureError, SyncFailure};
use futures::prelude::*;
use telegram_bot::{
    prelude::*,
    types::{Message, ParseMode},
    Error as TelegramError,
};

use super::Action;
use crate::state::State;
use crate::stats::TelegramToI64;

/// The action command name.
const CMD: &str = "all";

/// Whether the action is hidden.
const HIDDEN: bool = false;

/// The action help.
const HELP: &str = "Mention all members";

pub struct All;

impl All {
    pub fn new() -> Self {
        All
    }
}

#[async_trait]
impl Action for All {
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
        // Fetch the chat message stats
        let stats = match state.stats().fetch_chat_stats(
            state.db_connection(),
            msg.chat.id(),
            Some(msg.from.id),
        ) {
            Ok(stats) => stats,
            Err(e) => return Err(e.into()),
        };

        // Create a list of user mentions
        // TODO: limit mentions to 100 users max?
        // TODO: do not mention the bot itself
        // TODO: do not mention users not in this group anymore
        let mentions = stats
            .users()
            .iter()
            .filter(|(_, user_id, _, _, _)| *user_id != msg.from.id.to_i64())
            .map(|(_, user_id, _, _, _)| format!("[@](tg://user?id={})", user_id))
            .collect::<Vec<String>>()
            .join(" ");

        // Build a message future for sending the response
        state
            .telegram_send(
                msg.text_reply(format!(
                    "*Attention!* [{}](tg://user?id={}) mentions #all users.\n{}",
                    msg.from.first_name, msg.from.id, mentions,
                ))
                .parse_mode(ParseMode::Markdown),
            )
            .map_ok(|_| ())
            .map_err(|err| Error::Respond(SyncFailure::new(err)).into())
            .await
    }
}

/// A mention all action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
