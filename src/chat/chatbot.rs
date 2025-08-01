use indoc::indoc;
use openai_api_rs::v1::{api::OpenAIClient, chat_completion::*};
use serenity::all::{Message as SerenityMessage, *};
use sqlx::{Postgres, Transaction};
use tokio::sync::{Mutex, RwLock};

use crate::{
    Config,
    chat::{context, social},
    db::ChatMode,
};

pub async fn generate<'d>(
    transaction: &mut Transaction<'d, Postgres>,
    openai: &Mutex<OpenAIClient>,
    config: &RwLock<Config>,
    channel_id: &ChannelId,
    msg: &SerenityMessage,
    ctx: &Context,
    mentions_me: bool,
    chat_mode: ChatMode,
) -> eyre::Result<()> {
    let contexts = context::build(transaction, channel_id, config, chat_mode).await?;
    let should_reply =
        mentions_me || social::should_reply(contexts.social_context, openai, config).await?;
    if !should_reply {
        return Ok(());
    }
    let typing = channel_id.start_typing(&ctx.http);
    let response = generate_completion(contexts.chat_context, openai, config).await?;
    let response = &response.choices.first().unwrap().message;
    let Some(content) = &response.content else {
        return Ok(());
    };

    if content.trim().is_empty() {
        return Ok(());
    }

    let mut content = content.to_string();
    for (regex, replace) in &config.read().await.openrouter.chat.find_replace {
        content = regex.replace_all(&content, replace).to_string();
    }

    let Ok(reply) = msg
        .channel_id
        .send_message(
            &ctx,
            CreateMessage::new()
                .reference_message(msg)
                .content(content)
                .allowed_mentions(CreateAllowedMentions::new()),
        )
        .await
    else {
        return Ok(());
    };
    typing.stop();
    sqlx::query(indoc! {"
        WITH ensured_user AS (
            INSERT INTO users (id, name, display_name, is_self, is_bot)
            VALUES ($3, $4, $5, true, true)
            ON CONFLICT (id) DO UPDATE
            SET 
                name = EXCLUDED.name,
                display_name = EXCLUDED.display_name
        )
        INSERT INTO messages (
            id, mentions_me, sender, guild, channel, contents, reply
        ) VALUES (
            $2, true, $3, $6, $1, $7, $8
        );
    "})
    .bind(channel_id.get() as i64)
    .bind(reply.id.get() as i64)
    .bind(reply.author.id.get() as i64)
    .bind(&reply.author.name)
    .bind(reply.author.display_name())
    .bind(msg.guild_id.map(|id| id.get() as i64))
    .bind(reply.content_safe(&ctx))
    .bind(reply.referenced_message.map(|m| m.id.get() as i64))
    .execute(&mut **transaction)
    .await?;

    Ok(())
}

async fn generate_completion(
    context: Vec<ChatCompletionMessage>,
    openai: &Mutex<OpenAIClient>,
    config: &RwLock<Config>,
) -> eyre::Result<ChatCompletionResponse> {
    let body = {
        let config = &config.read().await.openrouter.chat;
        ChatCompletionRequest {
            model: config.model.to_owned(),
            max_tokens: None,
            temperature: Some(0.6_f64),
            top_p: Some(0.99_f64),
            n: Some(1),
            stream: Some(false),
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            messages: context,
            response_format: None,
            seed: None,
            tools: None,
            parallel_tool_calls: None,
            tool_choice: None,
            reasoning: config.reasoning.to_owned(),
        }
    };
    Ok(openai.lock().await.chat_completion(body).await?)
}
