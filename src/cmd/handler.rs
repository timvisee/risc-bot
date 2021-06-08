use regex::Regex;
use telegram_bot::types::Message;

use super::action::Error as ActionError;
use super::action::ACTIONS;
use crate::state::State;

lazy_static! {
    /// A regex for matching messages that contain a command.
    static ref CMD_REGEX: Regex = Regex::new(
        r"^/(?is)(?P<cmd>[A-Z0-9_]+)(@(?P<bot>[A-Z0-9_]+))?(\s.*$|$)",
    ).expect("failed to compile CMD_REGEX");
}

/// The command handler.
pub struct Handler;

impl Handler {
    /// Handle the given command.
    pub async fn handle(state: State, cmd: &str, msg: Message) -> Result<(), Error> {
        // Invoke the proper action
        let action = ACTIONS.iter().find(|a| a.is_cmd(cmd));
        if let Some(action) = action {
            // Build the action invocation future
            action.invoke(state, msg).await.map_err(|err| {
                Error::Cmd(ActionError::Invoke {
                    cause: err.compat(),
                    name: action.cmd().to_owned(),
                })
            })
        } else {
            Ok(())
        }
    }
}

/// Test wether the given message is recognized as a command.
/// If a specific bot is given with a `@bot` suffix, commands only match if the given bot name\
/// equals the name of this bot.
///
/// The actual command name is returned if it is, `None` otherwise.
// TODO: dynamically load the bot name from the API
pub fn matches_cmd(msg: &str) -> Option<&str> {
    match CMD_REGEX.captures(msg.trim()) {
        Some(groups) => {
            // Get the command
            let cmd = groups
                .name("cmd")
                .expect("failed to extract command from command regex")
                .as_str();

            // Ensure the bot name matches if one is given
            match groups.name("bot") {
                Some(bot) if bot.as_str() == "riscbot" => Some(cmd),
                None => Some(cmd),
                _ => None,
            }
        }
        None => None,
    }
}

/// A command handler error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while handling a command.
    #[fail(display = "failed to invoke command")]
    Cmd(#[cause] ActionError),
}

/// Convert command action errors to a command handler error.
impl From<ActionError> for Error {
    fn from(err: ActionError) -> Error {
        Error::Cmd(err)
    }
}
