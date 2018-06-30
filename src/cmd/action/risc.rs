use futures::{
    future::ok,
    Future,
};
use telegram_bot::{
    Api,
    prelude::*,
    types::{Message, ParseMode},
};

use app::{NAME, VERSION};
use super::Action;

/// The action command name.
const CMD: &'static str = "risc";

/// The action help.
const HELP: &'static str = "RISC info";

pub struct Risc;

impl Risc {
    pub fn new() -> Self {
        Risc
    }
}

impl Action for Risc {
    fn cmd(&self) -> &'static str {
        CMD
    }

    fn help(&self) -> &'static str {
        HELP
    }

    fn invoke(&self, msg: &Message, api: &Api) -> Box<Future<Item = (), Error = ()>> {
        api.spawn(
            msg.text_reply(format!(
                "\
                    `{} v{}`\n\
                    Developed by @timvisee\n\
                    https://timvisee.com/\
                ",
                NAME,
                VERSION,
            )).parse_mode(ParseMode::Markdown),
        );
        Box::new(ok(()))
    }
}