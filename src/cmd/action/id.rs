use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use failure::{err_msg, Compat, Error as FailureError, SyncFailure};
use futures::{
    future::{err, ok},
    Future,
};
use humansize::{file_size_opts, FileSize};
use humantime::format_duration;
use telegram_bot::{
    prelude::*,
    types::{
        ChannelPost, ForwardFrom, Message, MessageChat, MessageKind, MessageOrChannelPost,
        ParseMode, User,
    },
    Error as TelegramError,
};

use super::Action;
use state::State;

/// The action command name.
const CMD: &'static str = "id";

/// Whether the action is hidden.
const HIDDEN: bool = false;

/// The action help.
const HELP: &'static str = "Show hidden message/chat details";

pub struct Id;

impl Id {
    pub fn new() -> Self {
        Id
    }

    /// Build the info part for the given user.
    pub fn build_user_info(user: &User, caption: &str) -> String {
        // Build the header, add the ID
        let mut info = format!(
            "\
             *{}:*\n\
             ID: `{}`\
             ",
            caption, user.id,
        );

        // Apend the username if known
        if let Some(ref username) = user.username {
            info += &format!("\nUsername: `{}`", username,);
        }

        // Append the name
        if let Some(ref last_name) = user.last_name {
            info += &format!(
                "\n\
                 First name: _{}_\n\
                 Last name: _{}_\
                 ",
                user.first_name, last_name,
            );
        } else {
            info += &format!("\nName: _{}_", user.first_name,);
        }

        info
    }

    /// Build the info part for the given chat message.
    // TODO: the `build_channel_post_info` message is similar, merge them
    pub fn build_msg_info(msg: &Message, caption: &str) -> String {
        // Build the main info
        let mut info = format!(
            "\
             *{}:*\n\
             ID: `{}`\n\
             Poster ID: `{}`\n\
             Chat ID: `{}`\n\
             {}\n\
             Date: _{}_\
             ",
            caption,
            msg.id,
            msg.from.id,
            msg.chat.id(),
            Self::build_message_kind_details(&msg.kind),
            Self::format_timestamp(msg.date),
        );

        // Append the edit date if edited
        if let Some(date) = msg.edit_date {
            info += &format!("\nEdit date: _{}_", Self::format_timestamp(date),);
        }

        // Append the reply details
        info += &format!(
            "\nIs reply: _{}_",
            Self::format_yes_no(msg.reply_to_message.is_some()),
        );

        // Append forwarder information if availalbe
        if let Some(ref forward) = msg.forward {
            info += &format!(
                "\n\
                 Forwarded: _yes_\n\
                 Original date: _{}_\
                 ",
                Self::format_timestamp(forward.date),
            );

            // Add source information
            info += &match &forward.from {
                ForwardFrom::User { user } => format!("\nOriginal user ID: `{}`", user.id),
                ForwardFrom::Channel {
                    channel,
                    message_id,
                } => format!(
                    "\n\
                     Original message ID: `{}`\n\
                     Original channel ID: `{}`\
                     ",
                    message_id, channel.id,
                ),
                ForwardFrom::ChannelHiddenUser { sender_name } => {
                    format!("\nOriginal sender name: {}", sender_name)
                }
            };
        }

        info
    }

    /// Build the info part for the given channel post.
    pub fn build_channel_post_info(msg: &ChannelPost, caption: &str) -> String {
        // Build the main info
        let mut info = format!(
            "\
             *{}:*\n\
             ID: `{}`\n\
             Channel ID: `{}`\n\
             Channel title: _{}_\n\
             {}\n\
             Date: _{}_\
             ",
            caption,
            msg.id,
            msg.chat.id,
            msg.chat.title,
            Self::build_message_kind_details(&msg.kind),
            Self::format_timestamp(msg.date),
        );

        // Append the edit date if edited
        if let Some(date) = msg.edit_date {
            info += &format!("\nEdit date: _{}_", Self::format_timestamp(date),);
        }

        // Append the reply details
        info += &format!(
            "\nIs reply: _{}_",
            Self::format_yes_no(msg.reply_to_message.is_some()),
        );

        // Append forwarder information if availalbe
        if let Some(ref forward) = msg.forward {
            info += &format!(
                "\n\
                 Forwarded: _yes_\n\
                 Original date: _{}_\
                 ",
                Self::format_timestamp(forward.date),
            );

            // Add source information
            info += &match &forward.from {
                ForwardFrom::User { user } => format!("\nOriginal user ID: `{}`", user.id),
                ForwardFrom::Channel {
                    channel,
                    message_id,
                } => format!(
                    "\n\
                     Original message ID: `{}`\n\
                     Original channel ID: `{}`\
                     ",
                    message_id, channel.id,
                ),
                ForwardFrom::ChannelHiddenUser { sender_name } => {
                    format!("\nOriginal sender name: {}", sender_name)
                }
            };
        }

        info
    }

    /// Build the info part for the given chat message or channel post.
    pub fn build_msg_channel_post_info(msg: &MessageOrChannelPost, caption: &str) -> String {
        match msg {
            MessageOrChannelPost::Message(msg) => Self::build_msg_info(msg, caption),
            MessageOrChannelPost::ChannelPost(post) => Self::build_channel_post_info(post, caption),
        }
    }

    /// Build the info part for the given chat.
    pub fn build_chat_info(chat: &MessageChat, caption: &str) -> String {
        // Build the base info
        let mut info = format!("*{}:*", caption,);

        // Add details
        info += &match chat {
            MessageChat::Private(user) => format!(
                "\n\
                 Type: `private`\n\
                 Other user ID: `{}`\
                 ",
                user.id,
            ),
            MessageChat::Group(group) => format!(
                "\n\
                 Type: `group`\n\
                 Group ID: `{}`\n\
                 Title: _{}_\
                 ",
                group.id, group.title,
            ),
            MessageChat::Supergroup(group) => format!(
                "\n\
                 Type: `supergroup`\n\
                 Group ID: `{}`\n\
                 Title: _{}_\
                 ",
                group.id, group.title,
            ),
            MessageChat::Unknown(_) => "Type: `?`".into(),
        };

        info
    }

    /// Get `yes` or `no` if `true` or `false`.
    pub fn format_yes_no(b: bool) -> &'static str {
        if b {
            "yes"
        } else {
            "no"
        }
    }

    /// Format the given file size in a human readable format.
    pub fn format_file_size(size: &i64) -> String {
        size.file_size(file_size_opts::BINARY)
            .unwrap_or(format!("{} B", size))
    }

    /// Build message kind details.
    pub fn build_message_kind_details(kind: &MessageKind) -> String {
        match kind {
            MessageKind::Text { .. } => String::from("Kind: `text`"),
            MessageKind::Audio { data } => {
                // Build generic info
                let mut info = format!(
                    "\
                     Kind: `audio`\n\
                     Audio file ID: `{}`\n\
                     Audio length: _{}_\
                     ",
                    data.file_id,
                    format_duration(Duration::from_secs(data.duration as u64)),
                );

                // Append the performer name
                if let Some(ref performer) = data.performer {
                    info += &format!("\nAudio performer: _{}_", performer,);
                }

                // Append the title
                if let Some(ref title) = data.title {
                    info += &format!("\nAudio title: _{}_", title,);
                }

                // Append the mime type
                if let Some(ref mime) = data.mime_type {
                    info += &format!("\nAudio mime: `{}`", mime,);
                }

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!("\nAudio size: _{} b_", Self::format_file_size(size),);
                }

                info
            }
            MessageKind::Document { data, caption } => {
                // Build generic info
                let mut info = format!(
                    "\
                     Kind: `document`\n\
                     Document file ID: `{}`\
                     ",
                    data.file_id,
                );

                // Append the thumbnail information
                if let Some(ref thumb) = data.thumb {
                    info += &format!(
                        "\n\
                         Document thumb file ID: `{}`\n\
                         Document thumb pixels: _{}x{}_\
                         ",
                        thumb.file_id, thumb.width, thumb.height,
                    );

                    // Append the thumbnail file size
                    if let Some(ref size) = thumb.file_size {
                        info += &format!("\nDocument size: _{}_", Self::format_file_size(size),);
                    }
                }

                // Append the file name
                if let Some(ref name) = data.file_name {
                    info += &format!("\nDocument name: _{}_", name,);
                }

                // Append the mime type
                if let Some(ref mime) = data.mime_type {
                    info += &format!("\nDocument mime: _{}_", mime,);
                }

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!("\nDocument size: _{}_", Self::format_file_size(size),);
                }

                // Append the caption
                if let Some(caption) = caption {
                    info += &format!("\nDocument caption: _{}_", caption,);
                }

                info
            }
            MessageKind::Photo {
                data,
                caption,
                media_group_id,
            } => {
                // Build generic info
                let mut info = format!(
                    "\
                     Kind: `photo`\n\
                     Photos: `{}`\
                     ",
                    data.len(),
                );

                // Loop through the photos
                for (i, photo) in data.iter().enumerate() {
                    info += &format!(
                        "\n\
                         Photo #{} file ID: `{}`\n\
                         Photo #{} pixels: _{}x{}_\
                         ",
                        i, photo.file_id, i, photo.width, photo.height,
                    );

                    // Append the thumbnail file size
                    if let Some(ref size) = photo.file_size {
                        info +=
                            &format!("\nPhoto #{} size: _{}_", i, Self::format_file_size(size),);
                    }
                }

                // Append the media group
                if let Some(id) = media_group_id {
                    info += &format!("\nDocument media group ID: `{}`", id,);
                }

                // Append the caption
                if let Some(caption) = caption {
                    info += &format!("\nDocument caption: _{}_", caption,);
                }

                info
            }
            MessageKind::Sticker { data } => {
                // Build generic info
                let mut info = format!(
                    "\
                     Kind: `sticker`\n\
                     Sticker file ID: `{}`\n\
                     Sticker pixels: _{}x{}_\
                     ",
                    data.file_id, data.width, data.height,
                );

                // Append the thumbnail information
                if let Some(ref thumb) = data.thumb {
                    info += &format!(
                        "\n\
                         Sticker thumb file ID: `{}`\n\
                         Sticker thumb pixels: _{}x{}_\
                         ",
                        thumb.file_id, thumb.width, thumb.height,
                    );

                    // Append the thumbnail file size
                    if let Some(ref size) = thumb.file_size {
                        info +=
                            &format!("\nSticker thumb size: _{}_", Self::format_file_size(size),);
                    }
                }

                // Append the emojis
                if let Some(ref emoji) = data.emoji {
                    info += &format!("\nSticker emoji: _{}_", emoji,);
                }

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!("\nSticker size: _{}_", Self::format_file_size(size),);
                }

                info
            }
            MessageKind::Video {
                data,
                caption,
                media_group_id,
            } => {
                // Build generic info
                let mut info = format!(
                    "\
                     Kind: `video`\n\
                     Video file ID: `{}`\n\
                     Video pixels: _{}x{}_\n\
                     Video length: _{}_\
                     ",
                    data.file_id,
                    data.width,
                    data.height,
                    format_duration(Duration::from_secs(data.duration as u64)),
                );

                // Append the thumbnail information
                if let Some(ref thumb) = data.thumb {
                    info += &format!(
                        "\n\
                         Video thumb file ID: `{}`\n\
                         Video thumb pixels: _{}x{}_\
                         ",
                        thumb.file_id, thumb.width, thumb.height,
                    );

                    // Append the thumbnail file size
                    if let Some(ref size) = thumb.file_size {
                        info += &format!("\nVideo thumb size: _{}_", Self::format_file_size(size),);
                    }
                }

                // Append the mime type
                if let Some(ref mime) = data.mime_type {
                    info += &format!("\nVideo file mime: `{}`", mime,);
                }

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!("\nVideo size: _{}_", Self::format_file_size(size),);
                }

                // Append the media group
                if let Some(id) = media_group_id {
                    info += &format!("\nVideo media group ID: `{}`", id,);
                }

                // Append the caption
                if let Some(caption) = caption {
                    info += &format!("\nVideo caption: _{}_", caption,);
                }

                info
            }
            MessageKind::Voice { data } => {
                // Build generic info
                let mut info = format!(
                    "\
                     Kind: `voice`\n\
                     Voice file ID: `{}`\n\
                     Voice length: _{}_\
                     ",
                    data.file_id,
                    format_duration(Duration::from_secs(data.duration as u64)),
                );

                // Append the mime type
                if let Some(ref mime) = data.mime_type {
                    info += &format!("\nVoice file mime: `{}`", mime,);
                }

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!("\nVoice size: _{}_", Self::format_file_size(size),);
                }

                info
            }
            MessageKind::VideoNote { data } => {
                // Build generic info
                let mut info = format!(
                    "\
                     Kind: `video note`\n\
                     Note file ID: `{}`\n\
                     Note length: _{}_\
                     ",
                    data.file_id,
                    format_duration(Duration::from_secs(data.duration as u64)),
                );

                // Append the thumbnail information
                if let Some(ref thumb) = data.thumb {
                    info += &format!(
                        "\n\
                         Note thumb file ID: `{}`\n\
                         Note thumb pixels: _{}x{}_\
                         ",
                        thumb.file_id, thumb.width, thumb.height,
                    );

                    // Append the thumbnail file size
                    if let Some(ref size) = thumb.file_size {
                        info += &format!("\nNote thumb size: _{}_", Self::format_file_size(size),);
                    }
                }

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!("\nNote size: _{}_", Self::format_file_size(size),);
                }

                info
            }
            MessageKind::Contact { data } => {
                // Build generic info
                let mut info = format!(
                    "\
                     Kind: `contact`\n\
                     Contact phone: _{}_\n\
                     Contact first name: _{}_\
                     ",
                    data.phone_number, data.first_name,
                );

                // Append the last name
                if let Some(ref last_name) = data.last_name {
                    info += &format!("\nContact last name: _{}_", last_name,);
                }

                // Append the user ID
                if let Some(ref id) = data.user_id {
                    info += &format!("\nContact user ID: `{}`", id,);
                }

                info
            }
            MessageKind::Location { data } => format!(
                "\
                 Kind: `location`\n\
                 Location longitude: `{}`\n\
                 Location latitude: `{}`\
                 ",
                data.longitude, data.latitude,
            ),
            MessageKind::Venue { data } => {
                // Build generic info
                let mut info = format!(
                    "\
                     Kind: `venue`\n\
                     Venue loc longitude: `{}`\n\
                     Venue loc latitude: `{}`\n\
                     Venue title: _{}_\n\
                     Venue address: _{}_\
                     ",
                    data.location.longitude, data.location.latitude, data.title, data.address,
                );

                // Append the foursquare ID
                if let Some(ref id) = data.foursquare_id {
                    info += &format!("\nVenue foursquare ID: `{}`", id,);
                }

                info
            }
            MessageKind::NewChatMembers { data } => {
                // Build generic info
                let mut info = format!(
                    "\
                     Kind: `chat members joined`\n\
                     Joined users: _{}_\
                     ",
                    data.len(),
                );

                // Loop through the users
                for (i, user) in data.iter().enumerate() {
                    info += &format!("\nJoined user #{} ID: `{}`", i, user.id,);
                }

                info
            }
            MessageKind::LeftChatMember { data } => format!(
                "\
                 Kind: `chat member left`\n\
                 Left user ID: `{}`\n\
                 ",
                data.id,
            ),
            MessageKind::NewChatTitle { data } => format!(
                "\
                 Kind: `new chat title`\n\
                 New title: _{}_\
                 ",
                data,
            ),
            MessageKind::NewChatPhoto { data } => {
                // Build generic info
                let mut info = format!(
                    "\
                     Kind: `new chat photo`\n\
                     Photos: _{}_\
                     ",
                    data.len(),
                );

                // Append details for each photo
                for (i, photo) in data.iter().enumerate() {
                    // Add the photo details
                    info += &format!(
                        "\n\
                         Photo #{} file ID: `{}`\n\
                         Photo #{} pixels: _{}x{}_\
                         ",
                        i, photo.file_id, i, photo.width, photo.height,
                    );

                    // Append the file size
                    if let Some(ref size) = photo.file_size {
                        info +=
                            &format!("\nPhoto #{} size: _{}_", i, Self::format_file_size(size),);
                    }
                }

                info
            }
            MessageKind::DeleteChatPhoto => String::from("Kind: `deleted chat photo`"),
            MessageKind::GroupChatCreated => String::from("Kind: `created group chat`"),
            MessageKind::SupergroupChatCreated => String::from("Kind: `created supergroup chat`"),
            MessageKind::ChannelChatCreated => String::from("Kind: `created channel chat`"),
            MessageKind::MigrateToChatId { data } => format!(
                "\
                 Kind: `migrated to chat ID`\n\
                 New chat ID: `{}`\n\
                 ",
                data,
            ),
            MessageKind::MigrateFromChatId { data } => format!(
                "\
                 Kind: `migrated from chat ID`\n\
                 Old chat ID: `{}`\n\
                 ",
                data,
            ),
            MessageKind::PinnedMessage { .. } =>
            // TODO: describe the pinned message
            {
                String::from("Kind: `pinned message`")
            }
            MessageKind::Unknown { .. } => String::from("Kind: `?`"),
        }
    }

    /// Format the given timestamp in a humanly readable format.
    pub fn format_timestamp(timestamp: i64) -> String {
        DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(timestamp, 0), Utc).to_string()
    }
}

impl Action for Id {
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
        // Own the message and global state
        let msg = msg.clone();
        let state = state.clone();

        // Build a future to send a temporary response to claim an ID for the answer message
        let response = state
            .telegram_send(
                msg.text_reply("_Gathering facts..._")
                    .parse_mode(ParseMode::Markdown),
            )
            .map_err(|err| -> FailureError { SyncFailure::new(err).into() })
            .map_err(|err| Error::GatherFacts(err.compat()))
            .and_then(|msg_answer| {
                if let Some(msg_answer) = msg_answer {
                    ok(msg_answer)
                } else {
                    err(Error::GatherFacts(
                        err_msg("failed to gather facts, got empty response from Telegram API")
                            .compat(),
                    ))
                }
            });

        // Build the ID details message and update the answer
        let response = response
            .and_then(move |msg_answer| {
                // Build a list of info elements to print in the final message
                let mut info = Vec::new();

                // Information about the sender and his message
                info.push(Self::build_user_info(&msg.from, "You"));
                info.push(Self::build_msg_info(&msg, "Your message"));

                // Information about a quoted message by the sender
                if let Some(ref reply_to) = msg.reply_to_message {
                    info.push(Self::build_msg_channel_post_info(
                        reply_to,
                        "Your quoted message",
                    ));
                }

                // Information about the answer message and the chat
                info.push(Self::build_msg_info(&msg_answer, "This message"));
                info.push(Self::build_chat_info(&msg.chat, "This chat"));

                // Add information about the bot
                info.push(Self::build_user_info(&msg_answer.from, "Bot"));

                // Tell a user he may reply to an existing message
                if msg.reply_to_message.is_none() {
                    info.push(String::from(
                        "_Note: reply to an existing message with /id to show it's details._",
                    ));
                }

                // Build a future to update the temporary message with the actual ID response
                state
                    .telegram_send(
                        msg_answer
                            .edit_text(info.join("\n\n"))
                            .parse_mode(ParseMode::Markdown)
                            .disable_preview(),
                    )
                    .map(|_| ())
                    .map_err(|err| Error::Respond(SyncFailure::new(err)))
            })
            .from_err();

        Box::new(response)
    }
}

/// An ID action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending the first temporary response to gather facts.
    #[fail(display = "failed to send temporary response to gather ID facts")]
    GatherFacts(#[cause] Compat<FailureError>),

    /// An error occurred while sending the actual response by updating the temporary response
    /// message that was used for gathering facts.
    #[fail(display = "failed to update temporary response with actual ID details")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
