use indoc::indoc;
use openai_api_rs::v1::chat_completion::{Content as OpenAIContent, *};
use serenity::all::*;
use sqlx::{Postgres, Transaction};
use tokio::sync::RwLock;

use crate::{Config, chat::social::ShouldReply, db};

pub struct Contexts {
    pub chat_context: Vec<ChatCompletionMessage>,
    pub social_context: Vec<ChatCompletionMessage>,
}

pub async fn build<'d>(
    transaction: &mut Transaction<'d, Postgres>,
    channel_id: &ChannelId,
    config: &RwLock<Config>,
    chat_mode: db::ChatMode,
) -> eyre::Result<Contexts> {
    let chat_system_prompt: db::SystemPrompt = sqlx::query_as(indoc! {"
        SELECT sp.*
        FROM channels ch
        JOIN system_prompts sp ON sp.id = ch.system_prompt
        WHERE ch.id = $1;
    "})
    .bind(channel_id.get() as i64)
    .fetch_one(&mut **transaction)
    .await?;

    let social_system_prompt: db::SystemPrompt = sqlx::query_as(indoc! {"
        SELECT *
        FROM system_prompts
        WHERE id = 1;
    "})
    .fetch_one(&mut **transaction)
    .await?;

    let context: Vec<db::Message> = sqlx::query_as(indoc! {"
        SELECT m.*,
            rm.sender_name AS reply_sender_name,
            rm.contents AS reply_contents
        FROM messages m
        JOIN channels c ON c.id = m.channel
        LEFT JOIN messages rm ON rm.id = m.reply
        WHERE c.id = $1
            AND m.time > c.context_window
            AND (m.mentions_self IS TRUE OR $2 IS TRUE)
        ORDER BY m.id ASC;
    "})
    .bind(channel_id.get() as i64)
    .bind(chat_mode != db::ChatMode::MentionsOnly)
    .fetch_all(&mut **transaction)
    .await?;

    if context.len() >= config.read().await.openrouter.window_threshold {
        let middle_index = context.len() / 2;
        if middle_index < context.len() {
            let middle_message = context.get(middle_index).unwrap();
            sqlx::query(indoc! {"
                UPDATE channels
                SET context_window = $2
                WHERE id = $1;
            "})
            .bind(channel_id.get() as i64)
            .bind(middle_message.time as i64)
            .execute(&mut **transaction)
            .await?;
        }
    }

    let mut chat_context = vec![];
    let mut social_context = vec![];

    chat_context.push(ChatCompletionMessage {
        role: MessageRole::system,
        content: OpenAIContent::Text(chat_system_prompt.contents),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });

    social_context.push(ChatCompletionMessage {
        role: MessageRole::system,
        content: OpenAIContent::Text(social_system_prompt.contents),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });

    for message in context {
        let built_message = build_contents(&message);
        let (role, contents) = match message.is_self {
            true => (MessageRole::assistant, &message.contents),
            false => (MessageRole::user, &built_message),
        };

        chat_context.push(ChatCompletionMessage {
            role,
            content: OpenAIContent::Text(contents.to_owned()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        let social_serialized = serde_json::to_string(&ShouldReply {
            should_reply: message.is_self,
        })
        .unwrap();
        social_context.push(ChatCompletionMessage {
            role: MessageRole::assistant,
            content: OpenAIContent::Text(social_serialized),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        social_context.push(ChatCompletionMessage {
            role: MessageRole::user,
            content: OpenAIContent::Text(built_message),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    Ok(Contexts {
        chat_context,
        social_context,
    })
}

fn build_contents(message: &db::Message) -> String {
    let mut res = String::new();
    if let Some(reply_contents) = &message.reply_contents {
        let reply_sender_name = message
            .reply_sender_name
            .as_ref()
            .expect("Found reply contents but not name");
        let truncated_reply_contents = reply_contents
            .chars()
            .take(128)
            .map(|c| if c == '\n' { ' ' } else { c })
            .collect::<String>();
        res.push_str(&format!(
            "Replying to:\n\tReferenced Author ID: {}\n\tReferenced Truncated Contents: {}\n",
            reply_sender_name, truncated_reply_contents
        ));
    }
    res.push_str(&format!(
        "Author Name: {}\nAuthor ID: {}\nContents:\n{}",
        message.sender_display_name, message.sender_name, message.contents
    ));
    res
}
