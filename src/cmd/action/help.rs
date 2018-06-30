use futures::{
    future::ok,
    Future,
};
use telegram_bot::{
    Api,
    prelude::*,
    types::{Message, ParseMode},
};

use super::Action;
use super::super::handler::ACTIONS;

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

    fn invoke(&self, msg: &Message, api: &Api) -> Box<Future<Item = (), Error = ()>> {
        // Build the command list
        let mut cmds: Vec<String> = ACTIONS.iter()
            .filter(|action| !action.hidden())
            .map(|action| format!(
                "/{}: _{}_",
                action.cmd(),
                action.help(),
            ))
            .collect();
        cmds.sort();
        let cmd_list = cmds.join("\n");

        // Send the help message
        api.spawn(
            msg.text_reply(format!(
                "*RISC commands:*\n{}",
                cmd_list,
            ))
            .parse_mode(ParseMode::Markdown),
        );

        Box::new(ok(()))
    }
}
