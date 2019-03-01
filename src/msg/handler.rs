use failure::SyncFailure;
use futures::{future::ok, Future};
use regex::Regex;
use telegram_bot::{
    prelude::*,
    types::{Message, MessageChat, MessageKind, MessageOrChannelPost, ParseMode},
    Error as TelegramError,
};

use cmd::handler::{matches_cmd, Error as CmdHandlerError, Handler as CmdHandler};
use executor::isolated;
use state::State;

lazy_static! {
    /// A regex for matching messages that contain a Reddit reference.
    // TODO: two subreddit names with a space in between aren't matched
    static ref REDDIT_REGEX: Regex = Regex::new(
        r"(?:^|\s)(?i)/?r/(?P<r>[A-Z0-9_]{1,100})(?:$|\s)",
    ).expect("failed to compile REDDIT_REGEX");

    /// A regex for matching messages that contain sed syntax.
    static ref SED_REGEX: Regex = Regex::new(
        r"^\s*(s/.*/.*/)\s*$",
    ).expect("failed to compile SED_REGEX");
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

                // Handle sed messages
                if let Some(future) = Self::handle_sed(state, data, &msg) {
                    return Box::new(future.from_err());
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
            })
            .collect();
        reddits.sort_unstable();
        reddits.dedup();

        // If none, return
        if reddits.is_empty() {
            return None;
        }

        // Map the reddits into URLs
        let reddits: Vec<String> = reddits
            .iter()
            .map(|r| format!("[/r/{}](https://reddit.com/r/{})", r, r))
            .collect();

        // Send a response
        Some(
            state
                .telegram_send(
                    msg.text_reply(reddits.join("\n"))
                        .parse_mode(ParseMode::Markdown)
                        .disable_notification(),
                )
                .map(|_| ())
                .map_err(|err| Error::HandleReddit(SyncFailure::new(err))),
        )
    }

    /// Handle messages with sed syntax, such as: `s/foo/bar/`
    /// If the given message doesn't contain a sed-like command `None` is returned.
    pub fn handle_sed(
        state: &State,
        msg_text: &str,
        msg: &Message,
    ) -> Option<impl Future<Item = (), Error = SedError>> {
        // Attempt to collect a sed expression from the message, return None if there is none
        let expr: String = match SED_REGEX
            .captures(msg_text)
            .map(|r| {
                r.get(1)
                    .expect("failed to extract sed expr from SED_REGEX")
                    .as_str()
                    .to_owned()
            })
        {
            Some(expr) => expr,
            None => return None,
        };

        // Get the message text
        // TODO: clean this up!
        let reply = if let Some(reply) = &msg.reply_to_message {
            if let MessageOrChannelPost::Message(ref reply) = **reply {
                match &reply.kind {
                    MessageKind::Text { data, .. } => data.clone(),
                    _ => return None,
                }
            } else {
                return None;
            }
        } else {
            return None;
        };

        // Build the sed command to invoke
        let reply = reply.replace('\\', "\\\\").replace('\'', "\\\'");
        let cmd = format!("echo '{}' | sed {}", reply, expr);

        // Clone the state and message for in the processing future
        let state = state.clone();
        let msg = msg.clone();

        // Build the sed future, send the result when done
        let sed_future = isolated::execute_sync(cmd)
            .map_err(|_| SedError::Evaluate);
        let sed_future = sed_future.and_then(move |(output, status)| {

            // TODO: ensure the output is successful

            state
                .telegram_send(
                    msg.text_reply(&output)
                        .parse_mode(ParseMode::Markdown)
                        .disable_notification(),
                )
                .map(|_| ())
                .map_err(|err| SedError::Respond(SyncFailure::new(err)))

        });

        Some(sed_future)
    }

    /// Handle the give private/direct message.
    pub fn handle_private(state: &State, msg: &Message) -> impl Future<Item = (), Error = Error> {
        // Send a message to the user
        state
            .telegram_send(
                msg.text_reply(format!(
                    "`BLEEP BLOOP`\n`I AM A BOT`\n\n{}, direct messages are not supported yet.",
                    msg.from.first_name,
                ))
                .parse_mode(ParseMode::Markdown),
            )
            .map(|_| ())
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

    /// An error occurred while evaluating the sed expression.
    #[fail(display = "failed to process sed expression")]
    HandleSed(#[cause] SedError),

    /// An error occurred while processing a private message.
    #[fail(display = "failed to process private message")]
    HandlePrivate(#[cause] SyncFailure<TelegramError>),
}

impl From<CmdHandlerError> for Error {
    fn from(err: CmdHandlerError) -> Error {
        Error::HandleCmd(err)
    }
}

impl From<SedError> for Error {
    fn from(err: SedError) -> Error {
        Error::HandleSed(err)
    }
}

/// A message handler error.
#[derive(Debug, Fail)]
pub enum SedError {
    /// An error occurred while processing a Reddit message.
    #[fail(display = "failed to evaluate and run sed expression")]
    Evaluate,

    /// Failed to send the response message
    #[fail(display = "failed to send sed response")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
