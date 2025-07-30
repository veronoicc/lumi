use sqlx::{Decode, FromRow, Row, prelude::*};

pub struct SystemPrompt {
    pub id: i64,
    pub name: String,
    pub contents: String,
}

pub struct Channel {
    pub id: u64,
    pub chat_mode: ChatMode,
    pub context_window: u64,
    pub system_prompt: i64,
}

pub struct User {
    pub id: u64,
    pub name: String,
    pub display_name: String,
    pub is_self: bool,
    pub is_bot: bool,
}

pub struct Message {
    pub id: u64,
    pub mentions_me: bool,
    pub sender: u64,
    pub guild: Option<u64>,
    pub channel: u64,
    pub contents: String,
    pub reply: Option<u64>,
    pub time: u64,
}

impl<'r, R: Row> FromRow<'r, R> for SystemPrompt
where
    &'r str: sqlx::ColumnIndex<R>,
    i64: Decode<'r, R::Database>,
    i64: Type<R::Database>,
    String: Decode<'r, R::Database>,
    String: Type<R::Database>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            contents: row.try_get("contents")?,
        })
    }
}

impl<'r, R: Row> FromRow<'r, R> for Channel
where
    &'r str: sqlx::ColumnIndex<R>,
    i64: Decode<'r, R::Database>,
    i64: Type<R::Database>,
    ChatMode: Decode<'r, R::Database>,
    ChatMode: Type<R::Database>,
    bool: Decode<'r, R::Database>,
    bool: Type<R::Database>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get::<i64, _>("id")? as _,
            chat_mode: row.try_get("chat_mode")?,
            context_window: row.try_get::<i64, _>("context_window")? as _,
            system_prompt: row.try_get("system_prompt")?,
        })
    }
}

impl<'r, R: Row> FromRow<'r, R> for User
where
    &'r str: sqlx::ColumnIndex<R>,
    i64: Decode<'r, R::Database>,
    i64: Type<R::Database>,
    String: Decode<'r, R::Database>,
    String: Type<R::Database>,
    bool: Decode<'r, R::Database>,
    bool: Type<R::Database>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get::<i64, _>("id")? as _,
            name: row.try_get("name")?,
            display_name: row.try_get("display_name")?,
            is_self: row.try_get("is_self")?,
            is_bot: row.try_get("is_bot")?,
        })
    }
}

impl<'r, R: Row> FromRow<'r, R> for Message
where
    &'r str: sqlx::ColumnIndex<R>,
    i64: Decode<'r, R::Database>,
    i64: Type<R::Database>,
    bool: Decode<'r, R::Database>,
    bool: Type<R::Database>,
    String: Decode<'r, R::Database>,
    String: Type<R::Database>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get::<i64, _>("id")? as _,
            mentions_me: row.try_get("mentions_me")?,
            sender: row.try_get::<i64, _>("sender")? as _,
            guild: row.try_get::<Option<i64>, _>("guild")?.map(|v| v as _),
            channel: row.try_get::<i64, _>("channel")? as _,
            contents: row.try_get("contents")?,
            reply: row.try_get::<Option<i64>, _>("reply")?.map(|v| v as _),
            time: row.try_get::<i64, _>("time")? as _,
        })
    }
}

#[derive(Debug, sqlx::Type, serde::Serialize, serde::Deserialize, PartialEq)]
#[sqlx(type_name = "chat_mode", rename_all = "snake_case")]
pub enum ChatMode {
    FreeResponse,
    MentionsOnly,
    MentionsOnlyAllContext,
}

impl ToString for ChatMode {
    fn to_string(&self) -> String {
        match self {
            ChatMode::FreeResponse => "Free Response",
            ChatMode::MentionsOnly => "Mentions Only",
            ChatMode::MentionsOnlyAllContext => "Mentions Only All Context",
        }
        .to_string()
    }
}

impl std::str::FromStr for ChatMode {
    type Err = eyre::Report;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "free_response" => Self::FreeResponse,
            "mentions_only" => Self::MentionsOnly,
            "mentions_only_all_context" => Self::MentionsOnlyAllContext,
            _ => {
                eyre::bail!("Invalid chat mode string");
            }
        })
    }
}
