use chrono::NaiveDateTime;

use crate::schema::{chat, chat_user_stats, user};

#[derive(Queryable, Identifiable)]
#[primary_key(telegram_id)]
#[table_name = "chat"]
pub struct Chat {
    pub telegram_id: i64,
    pub title: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Identifiable)]
#[primary_key(telegram_id)]
#[table_name = "user"]
pub struct User {
    pub telegram_id: i64,
    pub username: Option<String>,
    pub first_name: String,
    pub last_name: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Identifiable)]
#[primary_key(chat_id, user_id, message_type)]
#[table_name = "chat_user_stats"]
pub struct ChatUserStats {
    pub chat_id: i64,
    pub user_id: i64,
    pub message_type: i16,
    pub messages: i32,
    pub edits: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
