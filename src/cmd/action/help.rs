use failure::{Error as FailureError, SyncFailure};
use futures::Future;
use telegram_bot::{
    prelude::*,
    types::{Message, ParseMode},
    Error as TelegramError,
};

use super::{Action, ACTIONS};
use state::State;

/// The action command name.
const CMD: &'static str = "help";

/// Whether the action is hidden.
const HIDDEN: bool = false;

/// The action help.
const HELP: &'static str = "Show help";

pub struct Help;

impl Help {
    pub fn new() -> Self {
        Help
    }
}

impl Action for Help {
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
        // Build the command list
        let cmd_list = build_help_list();

        // Build a future for sending the response help message
        let future = state
            .telegram_send(
                msg.text_reply(format!("*RISC commands:*\n{}", cmd_list,))
                    .parse_mode(ParseMode::Markdown),
            )
            .map(|_| ())
            .map_err(|err| Error::Respond(SyncFailure::new(err)))
            .from_err();

        Box::new(future)
    }
}

/// Build a string with a list of help commands.
pub(crate) fn build_help_list() -> String {
    let mut cmds: Vec<String> = ACTIONS
        .iter()
        .filter(|action| !action.hidden())
        .map(|action| format!("/{}: _{}_", action.cmd(), action.help(),))
        .collect();
    cmds.sort();
    cmds.join("\n")
}

/// A help action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
