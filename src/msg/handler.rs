use failure::SyncFailure;
use futures::{future::ok, Future};
use regex::Regex;
use telegram_bot::{
    prelude::*,
    types::{Message, MessageChat, MessageKind, ParseMode},
    Error as TelegramError,
};

use cmd::handler::{matches_cmd, Error as CmdHandlerError, Handler as CmdHandler};
use state::State;

lazy_static! {
    /// A regex for matching messages that contain a Reddit reference.
    // TODO: two subreddit names with a space in between aren't matched
    static ref REDDIT_REGEX: Regex = Regex::new(
        r"(?:^|\s)(?i)/?r/(?P<r>[A-Z0-9_]{1,100})(?:$|\s)",
    ).expect("failed to compile REDDIT_REGEX");
}

/// The generic message handler.
/// This handler should process all incomming messages from Telegram,
/// and route them to the proper actions.
pub struct Handler;

impl Handler {
    /// Handle the given message.
    pub fn handle(state: &State, msg: Message) -> Box<Future<Item = (), Error = Error>> {
        match &msg.kind {
            MessageKind::Text { ref data, .. } => {
                // Log all incomming text messages
                println!(
                    "MSG <{}>@{}: {}",
                    &msg.from.first_name,
                    &msg.chat.id(),
                    data,
                );

                // Route the message to the command handler, if it's a command
                if let Some(cmd) = matches_cmd(data) {
                    return Box::new(CmdHandler::handle(state, cmd, msg.clone()).from_err());
                }

                // Handle Reddit messages
                if let Some(future) = Self::handle_reddit(state, data, &msg) {
                    return Box::new(future);
                }

                // Route private messages
                match &msg.chat {
                    MessageChat::Private(..) => return Box::new(Self::handle_private(state, &msg)),
                    _ => {}
                }
            }
            _ => {}
        }

        Box::new(ok(()))
    }

    /// Handle messages with Reddit references, such as messages containing `/r/rust`.
    /// If the given message does not contain any Reddit Reference, `None` is returned.
    pub fn handle_reddit(
        state: &State,
        msg_text: &str,
        msg: &Message,
    ) -> Option<impl Future<Item = (), Error = Error>> {
        // Collect all reddits from the message
        let mut reddits: Vec<String> = REDDIT_REGEX
            .captures_iter(msg_text)
            .map(|r| {
                r.name("r")
                    .expect("failed to extract r from REDDIT_REGEX")
                    .as_str()
                    .to_owned()
            }).collect();
        reddits.sort_unstable();
        reddits.dedup();

        // If none, return
        if reddits.is_empty() {
            return None;
        }

        // Map the reddits into URLs
        let reddits: Vec<String> = reddits
            .iter()
            .map(|r| format!("[/r/{}](https://old.reddit.com/r/{})", r, r))
            .collect();

        // Send a response
        Some(
            state
                .telegram_send(
                    msg.text_reply(reddits.join("\n"))
                        .parse_mode(ParseMode::Markdown)
                        .disable_notification(),
                ).map(|_| ())
                .map_err(|err| Error::HandleReddit(SyncFailure::new(err))),
        )
    }

    /// Handle the give private/direct message.
    pub fn handle_private(state: &State, msg: &Message) -> impl Future<Item = (), Error = Error> {
        // Send a message to the user
        state
            .telegram_send(
                msg.text_reply(format!(
                    "`BLEEP BLOOP`\n`I AM A BOT`\n\n{}, direct messages are not supported yet.",
                    msg.from.first_name,
                )).parse_mode(ParseMode::Markdown),
            ).map(|_| ())
            .map_err(|err| Error::HandlePrivate(SyncFailure::new(err)))
    }
}

/// A message handler error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while processing a command.
    #[fail(display = "failed to process command message")]
    HandleCmd(#[cause] CmdHandlerError),

    /// An error occurred while processing a Reddit message.
    #[fail(display = "failed to process reddit message")]
    HandleReddit(#[cause] SyncFailure<TelegramError>),

    /// An error occurred while processing a private message.
    #[fail(display = "failed to process private message")]
    HandlePrivate(#[cause] SyncFailure<TelegramError>),
}

/// Convert a command handler error into a message handling error.
impl From<CmdHandlerError> for Error {
    fn from(err: CmdHandlerError) -> Error {
        Error::HandleCmd(err)
    }
}
