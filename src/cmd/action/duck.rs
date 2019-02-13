use failure::{Error as FailureError, SyncFailure};
use futures::{future::ok, Future};
use htmlescape;
use telegram_bot::{
    prelude::*,
    types::{Message, MessageKind, ParseMode},
    Error as TelegramError,
};
use urlencoding;

use super::Action;
use state::State;

/// The action command name.
const CMD: &'static str = "duck";

/// Whether the action is hidden.
const HIDDEN: bool = true;

/// The action help.
const HELP: &'static str = "Create DuckDuckGo search URL";

/// Base URL, to append the search query to.
const URL: &'static str = "https://duckduckgo.com/?q=";

pub struct Duck;

impl Duck {
    pub fn new() -> Self {
        Duck
    }
}

impl Action for Duck {
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
        if let MessageKind::Text { ref data, .. } = &msg.kind {
            // Get the user's input
            // TODO: actually properly fetch the user input
            let input = data
                .splitn(2, ' ')
                .skip(1)
                .next()
                .map(|cmd| cmd.trim_left())
                .unwrap_or("")
                .trim()
                .to_owned();

            // Make sure something was entered
            if input.is_empty() {
                // Build a message future for sending the response
                let future = state
                    .telegram_send(msg.text_reply("Please provide a search query, such as:\n\n`/duck Telegram`\n`/duck !w Telegram app`").parse_mode(ParseMode::Markdown))
                    .map(|_| ())
                    .map_err(|err| Error::Respond(SyncFailure::new(err)))
                    .from_err();

                return Box::new(future);
            }

            // Build the search URL
            let url = format!("{}{}", URL, urlencoding::encode(&input));

            // Build the response string
            let response = format!("<a href=\"{}\">{}</a>", url, htmlescape::encode_minimal(&input));

            // Build a future for sending the response message
            let future = state
                .telegram_send(msg.text_reply(input).parse_mode(ParseMode::Html))
                .map(|_| ())
                .map_err(|err| Error::Respond(SyncFailure::new(err)))
                .from_err();

            Box::new(future)
        } else {
            Box::new(ok(()))
        }
    }
}

/// A duck action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}