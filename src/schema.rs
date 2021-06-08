table! {
    chat (telegram_id) {
        telegram_id -> Bigint,
        title -> Nullable<Varchar>,
        created_at -> Datetime,
        updated_at -> Datetime,
    }
}

table! {
    chat_user_stats (chat_id, user_id, message_type) {
        chat_id -> Bigint,
        user_id -> Bigint,
        message_type -> Smallint,
        messages -> Integer,
        edits -> Integer,
        created_at -> Datetime,
        updated_at -> Datetime,
    }
}

table! {
    user (telegram_id) {
        telegram_id -> Bigint,
        username -> Nullable<Text>,
        first_name -> Text,
        last_name -> Nullable<Text>,
        created_at -> Datetime,
        updated_at -> Datetime,
    }
}

joinable!(chat_user_stats -> chat (chat_id));
joinable!(chat_user_stats -> user (user_id));

allow_tables_to_appear_in_same_query!(
    chat,
    chat_user_stats,
    user,
);
