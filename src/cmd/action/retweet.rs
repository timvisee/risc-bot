use failure::{Error as FailureError, SyncFailure};
use futures::{future::ok, Future};
use regex::Regex;
use telegram_bot::{
    prelude::*,
    types::{Message, MessageChat, MessageKind, MessageOrChannelPost, ParseMode},
    Error as TelegramError,
};

use super::help::build_help_list;
use super::Action;
use state::State;

/// The action command name.
const CMD: &'static str = "rt";

/// Whether the action is hidden.
const HIDDEN: bool = false;

/// The action help.
const HELP: &'static str = "Retweet a message";

lazy_static! {
    /// A regex for matching a retweeted message
    static ref RT_REGEX: Regex = Regex::new(
        r"^.* RTs:\s(?P<msg>.+)$",
    ).expect("failed to compile RT_REGEX");
}

pub struct Retweet;

impl Retweet {
    pub fn new() -> Self {
        Retweet
    }
}

impl Action for Retweet {
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
        // Get the reply message which we should retweet
        let retweet_msg: &Message = match msg.reply_to_message {
            Some(ref msg) => match msg.as_ref() {
                MessageOrChannelPost::Message(msg) => msg,
                MessageOrChannelPost::ChannelPost(_) => {
                    return Box::new(
                        state
                            .telegram_send(
                                msg.text_reply(format!("You can't retweet a channel post."))
                                    .parse_mode(ParseMode::Markdown),
                            )
                            .map(|_| ())
                            .map_err(|err| Error::Respond(SyncFailure::new(err)))
                            .from_err(),
                    );
                }
            },
            None => {
                return Box::new(
                    state
                        .telegram_send(
                            msg.text_reply(format!(
                                "\
                                 To retweet, you must reply to a message with the `/{}` command.\
                                 ",
                                CMD,
                            ))
                            .parse_mode(ParseMode::Markdown),
                        )
                        .map(|_| ())
                        .map_err(|err| Error::Respond(SyncFailure::new(err)))
                        .from_err(),
                );
            }
        };

        // Only text messages can be retweeted
        match &retweet_msg.kind {
            MessageKind::Text { data, .. } => {
                // Get the retweet text
                let mut retweet_text = data.clone();

                // Remove any previous retweet notices
                match RT_REGEX.captures(&data) {
                    Some(groups) => {
                        retweet_text = groups
                            .name("msg")
                            .expect("failed to extract message from retweet target")
                            .as_str()
                            .to_owned();
                    }
                    None => {}
                }

                // Prefix a newline if the retweet text is multi line
                if retweet_text.contains("\n") {
                    retweet_text.insert(0, '\n');
                }

                // Send the retweet message
                Box::new(
                    state
                        .telegram_send(
                            retweet_msg
                                .text_reply(format!(
                                    "\
                                     <a href=\"tg://user?id={}\">{}</a> <b>RTs:</b> {}",
                                    msg.from.id, msg.from.first_name, retweet_text,
                                ))
                                .parse_mode(ParseMode::Html),
                        )
                        .map(|_| ())
                        .map_err(|err| Error::Respond(SyncFailure::new(err)))
                        .from_err(),
                )
            }
            _ => Box::new(
                state
                    .telegram_send(
                        msg.text_reply(format!(
                            "Only text messages can be retweeted at this moment."
                        ))
                        .parse_mode(ParseMode::Markdown),
                    )
                    .map(|_| ())
                    .map_err(|err| Error::Respond(SyncFailure::new(err)))
                    .from_err(),
            ),
        }
    }
}

/// A start action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
