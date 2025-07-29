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

    if content.is_empty() {
        return Ok(());
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
        INSERT INTO messages (
            id, is_self, mentions_self, sender, sender_name, sender_display_name, guild, channel, contents, reply
        ) VALUES (
            $2, true, true, $3, $4, $5, $6, $1, $7, $8
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
