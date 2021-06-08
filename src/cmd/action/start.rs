use async_trait::async_trait;
use failure::{Error as FailureError, SyncFailure};
use futures::prelude::*;
use telegram_bot::{
    prelude::*,
    types::{Message, MessageChat, MessageKind, ParseMode},
    Error as TelegramError,
};

use super::help::build_help_list;
use super::Action;
use crate::state::State;

/// The action command name.
const CMD: &str = "start";

/// Whether the action is hidden.
const HIDDEN: bool = true;

/// The action help.
const HELP: &str = "Start using RISC";

pub struct Start;

impl Start {
    pub fn new() -> Self {
        Start
    }
}

#[async_trait]
impl Action for Start {
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
        // Do not respond in non-private chats
        if let MessageKind::Text { .. } = &msg.kind {
            match &msg.chat {
                MessageChat::Private(..) => {}
                _ => return Ok(()),
            }
        }

        // Build a future for sending the response start message
        state
            .telegram_send(
                msg.text_reply(format!(
                    "\
                            *Welcome {}!*\n\
                            \n\
                            This bot adds useful features to Telegram such as message stats \
                            tracking, and is intended to be used in group chats. \
                            Add @riscbot to a group chat to start using it.\n\
                            \n\
                            You may choose one of the following commands to try it out:\n\
                            \n\
                            {}
                        ",
                    msg.from.first_name,
                    build_help_list(),
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
