use telegram_bot::types::{ChannelPost, Message, MessageKind, MessageOrChannelPost};

/// A trait to obtain text from a message.
pub trait MessageText {
    /// Obtain text from a message if available.
    fn text(&self) -> Option<String>;
}

impl MessageText for MessageOrChannelPost {
    fn text(&self) -> Option<String> {
        match self {
            MessageOrChannelPost::Message(msg) => msg.text(),
            MessageOrChannelPost::ChannelPost(post) => post.text(),
        }
    }
}

impl MessageText for Message {
    fn text(&self) -> Option<String> {
        self.kind.text()
    }
}

impl MessageText for MessageKind {
    fn text(&self) -> Option<String> {
        match self {
            MessageKind::Text { data, .. } => Some(data.clone()),
            MessageKind::Audio { data } => data.title.clone(),
            MessageKind::Document { data, caption } => {
                caption.clone().or_else(|| data.file_name.clone())
            }
            MessageKind::Photo { caption, .. } => caption.clone(),
            MessageKind::Sticker { .. } => None,
            MessageKind::Video { caption, .. } => caption.clone(),
            MessageKind::Voice { .. } => None,
            MessageKind::VideoNote { .. } => None,
            MessageKind::Contact { data } => Some(data.first_name.clone()),
            MessageKind::Location { .. } => None,
            MessageKind::Venue { data } => Some(data.title.clone()),
            MessageKind::NewChatMembers { .. } => None,
            MessageKind::LeftChatMember { .. } => None,
            MessageKind::NewChatTitle { data } => Some(data.clone()),
            MessageKind::NewChatPhoto { .. } => None,
            MessageKind::DeleteChatPhoto => None,
            MessageKind::GroupChatCreated => None,
            MessageKind::SupergroupChatCreated => None,
            MessageKind::ChannelChatCreated => None,
            MessageKind::MigrateToChatId { .. } => None,
            MessageKind::MigrateFromChatId { .. } => None,
            MessageKind::PinnedMessage { data } => data.text(),
            MessageKind::Poll { data } => Some(data.question.clone()),
            MessageKind::Unknown { .. } => None,
        }
    }
}

impl MessageText for ChannelPost {
    fn text(&self) -> Option<String> {
        self.kind.text()
    }
}
