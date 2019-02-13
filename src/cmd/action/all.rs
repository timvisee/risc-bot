use failure::{Error as FailureError, SyncFailure};
use futures::{future::err, Future};
use telegram_bot::{
    prelude::*,
    types::{Message, ParseMode},
    Error as TelegramError,
};

use super::Action;
use state::State;
use stats::TelegramToI64;

/// The action command name.
const CMD: &'static str = "all";

/// Whether the action is hidden.
const HIDDEN: bool = false;

/// The action help.
const HELP: &'static str = "Mention all members";

pub struct All;

impl All {
    pub fn new() -> Self {
        All
    }
}

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

    fn invoke(&self, state: &State, msg: &Message) -> Box<Future<Item = (), Error = FailureError>> {
        // Fetch the chat message stats
        let stats =
            match state
                .stats()
                .fetch_chat_stats(state.db(), msg.chat.id(), Some(msg.from.id))
            {
                Ok(stats) => stats,
                Err(e) => return Box::new(err(e.into())),
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
        let future = state
            .telegram_send(
                msg.text_reply(format!(
                    "*Attention!* [{}](tg://user?id={}) mentions #all users.\n{}",
                    msg.from.first_name, msg.from.id, mentions,
                ))
                .parse_mode(ParseMode::Markdown),
            )
            .map(|_| ())
            .map_err(|err| Error::Respond(SyncFailure::new(err)))
            .from_err();

        Box::new(future)
    }
}

/// A mention all action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
