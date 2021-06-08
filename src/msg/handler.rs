use failure::SyncFailure;
use regex::Regex;
use telegram_bot::{
    prelude::*,
    types::{Message, MessageChat, MessageKind, ParseMode},
    Error as TelegramError,
};

use crate::cmd::handler::{matches_cmd, Error as CmdHandlerError, Handler as CmdHandler};
use crate::executor::isolated;
use crate::state::State;
use crate::traits::MessageText;

lazy_static! {
    /// A regex for matching messages that contain a Reddit reference.
    // TODO: two subreddit names with a space in between aren't matched
    static ref REDDIT_REGEX: Regex = Regex::new(
        r"(?:^|\s)(?i)/?r/(?P<r>[A-Z0-9_]{1,100})(?:$|\s)",
    ).expect("failed to compile REDDIT_REGEX");

    /// A regex for matching messages that contain sed syntax.
    static ref SED_REGEX: Regex = Regex::new(
        r"^\s*([sy]/.*/.*/[a-zA-Z0-9]*)\s*$",
    ).expect("failed to compile SED_REGEX");

    /// A regex for matching messages that contain tr syntax.
    static ref TR_REGEX: Regex = Regex::new(
        r"^\s*tr\s+(.*\s*.*)\s*$",
    ).expect("failed to compile TR_REGEX");
}

/// The generic message handler.
/// This handler should process all incomming messages from Telegram,
/// and route them to the proper actions.
pub struct Handler;

impl Handler {
    /// Handle the given message.
    pub async fn handle(state: State, msg: Message) -> Result<(), Error> {
        if let MessageKind::Text { ref data, .. } = &msg.kind {
            // Log all incomming text messages
            println!(
                "MSG <{}>@{}: {}",
                &msg.from.first_name,
                &msg.chat.id(),
                data,
            );

            // Route the message to the command handler, if it's a command
            if let Some(cmd) = matches_cmd(data) {
                return CmdHandler::handle(state.clone(), cmd, msg.clone())
                    .await
                    .map_err(Error::HandleCmd);
            }

            // Handle Reddit messages
            if let Some(future) = Self::handle_reddit(&state, data, &msg).await {
                return future;
            }

            // Handle sed messages
            if let Some(future) = Self::handle_sed(&state, data, &msg).await {
                return future.map_err(Error::HandleSed);
            }

            // Handle tr messages
            if let Some(future) = Self::handle_tr(&state, data, &msg).await {
                return future.map_err(Error::HandleTr);
            }

            // Route private messages
            if let MessageChat::Private(..) = &msg.chat {
                return Self::handle_private(&state, &msg).await;
            }
        }

        Ok(())
    }

    /// Handle messages with Reddit references, such as messages containing `/r/rust`.
    /// If the given message does not contain any Reddit Reference, `None` is returned.
    pub async fn handle_reddit(
        state: &State,
        msg_text: &str,
        msg: &Message,
    ) -> Option<Result<(), Error>> {
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
            .map(|r| format!("[/r/{}](https://old.reddit.com/r/{})", r, r))
            .collect();

        // Send a response
        Some(
            state
                .telegram_send(
                    msg.text_reply(reddits.join("\n"))
                        .parse_mode(ParseMode::Markdown)
                        .disable_notification(),
                )
                .await
                .map(|_| ())
                .map_err(|err| Error::HandleReddit(SyncFailure::new(err))),
        )
    }

    /// Handle messages with sed syntax, such as: `s/foo/bar/`
    /// If the given message doesn't contain a sed-like command `None` is returned.
    pub async fn handle_sed(
        state: &State,
        msg_text: &str,
        msg: &Message,
    ) -> Option<Result<(), SedError>> {
        // Attempt to collect a sed expression from the message, return None if there is none
        let expr: String = match SED_REGEX.captures(msg_text).map(|r| {
            r.get(1)
                .expect("failed to extract sed expr from SED_REGEX")
                .as_str()
                .to_owned()
        }) {
            Some(expr) => expr,
            None => return None,
        };

        // Get the message text
        let reply = match msg.reply_to_message.as_ref().and_then(|m| m.text()) {
            Some(reply) => reply,
            None => return None,
        };

        // Build the sed command to invoke
        let expr = expr.replace('\'', "'\"'\"'");
        let reply = reply.replace('\'', "'\"'\"'");
        let cmd = format!("echo '{}' | sed '{}'", reply, expr);

        // Clone the state and message for in the processing future
        let state = state.clone();
        let msg = msg.clone();

        // Run sed, gather results
        let sed = isolated::execute_sync(cmd)
            .await
            .map_err(|_| SedError::Evaluate);
        let (mut output, status) = match sed {
            Ok(sed) => (sed.0, sed.1),
            Err(_) => return Some(Err(SedError::Evaluate)),
        };

        // Prefix an error message on failure
        if !status.success() {
            output.insert_str(0, "Failed to evaluate sed expression:\n\n");
        }

        // Send the response
        Some(
            state
                .telegram_send(msg.text_reply(&output).disable_notification())
                .await
                .map(|_| ())
                .map_err(|err| SedError::Respond(SyncFailure::new(err))),
        )
    }

    /// Handle messages with tr syntax, such as: `tr a b`
    /// If the given message doesn't contain a tr-like command `None` is returned.
    pub async fn handle_tr(
        state: &State,
        msg_text: &str,
        msg: &Message,
    ) -> Option<Result<(), TrError>> {
        // Attempt to collect a tr expression from the message, return None if there is none
        let expr: String = match TR_REGEX.captures(msg_text).map(|r| {
            r.get(1)
                .expect("failed to extract tr expr from TR_REGEX")
                .as_str()
                .to_owned()
        }) {
            Some(expr) => expr,
            None => return None,
        };

        // Get the message text
        let reply = match msg.reply_to_message.as_ref().and_then(|m| m.text()) {
            Some(reply) => reply,
            None => return None,
        };

        // Build the tr command to invoke
        let expr = expr.replace('\'', "'\"'\"'");
        let reply = reply.replace('\'', "'\"'\"'");
        let cmd = format!("echo '{}' | tr {}", reply, expr);

        // Clone the state and message for in the processing future
        let state = state.clone();
        let msg = msg.clone();

        // Run tr, gather results
        let tr = isolated::execute_sync(cmd).await;
        let (mut output, status) = match tr {
            Ok(tr) => (tr.0, tr.1),
            Err(_) => return Some(Err(TrError::Evaluate)),
        };

        // Prefix an error message on failure
        if !status.success() {
            output.insert_str(0, "Failed to evaluate tr expression:\n\n");
        }

        // Send the response
        Some(
            state
                .telegram_send(msg.text_reply(&output).disable_notification())
                .await
                .map(|_| ())
                .map_err(|err| TrError::Respond(SyncFailure::new(err))),
        )
    }

    /// Handle the give private/direct message.
    pub async fn handle_private(state: &State, msg: &Message) -> Result<(), Error> {
        // Send a message to the user
        state
            .telegram_send(
                msg.text_reply(format!(
                    "`BLEEP BLOOP`\n`I AM A BOT`\n\n{}, direct messages are not supported yet.",
                    msg.from.first_name,
                ))
                .parse_mode(ParseMode::Markdown),
            )
            .await
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

    /// An error occurred while evaluating the tr expression.
    #[fail(display = "failed to process tr expression")]
    HandleTr(#[cause] TrError),

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

impl From<TrError> for Error {
    fn from(err: TrError) -> Error {
        Error::HandleTr(err)
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

/// A message handler error.
#[derive(Debug, Fail)]
pub enum TrError {
    /// An error occurred while processing a Reddit message.
    #[fail(display = "failed to evaluate and run tr expression")]
    Evaluate,

    /// Failed to send the response message
    #[fail(display = "failed to send tr response")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
