use indoc::indoc;
use openai_api_rs::v1::api::OpenAIClient;
use serenity::{all::Message as SerenityMessage, all::*, async_trait};
use sqlx::PgPool;
use tokio::sync::{Mutex, RwLock};

use crate::{Config, chat::chatbot, commands, db};

pub struct Handler {
    pub config: RwLock<Config>,
    pub openai: Mutex<OpenAIClient>,
    pub db: PgPool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if reaction.message_author_id == Some(ctx.cache.current_user().id)
            && reaction.emoji.unicode_eq("\u{274C}")
        {
            let msg = match reaction.message(&ctx).await {
                Ok(msg) => msg,
                Err(err) => {
                    println!("Error fetching message: {err:?}");
                    return;
                }
            };
            if let Err(err) = msg.delete(&ctx).await {
                println!("Error deleting message: {err:?}");
            };
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        Command::set_global_commands(
            &ctx,
            vec![
                commands::reload::register(),
                commands::reset_context::register(),
                commands::system_prompt::register(),
                commands::chat_mode::register(),
            ],
        )
        .await
        .expect("Error setting global commands");
        println!("Bot ready as {}", ready.user.name);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let res = match command.data.name.as_str() {
                "reload" => commands::reload::run(&ctx, &command, &self).await,
                "reset_context" => commands::reset_context::run(&ctx, &command, &self).await,
                "system_prompt" => commands::system_prompt::run(&ctx, &command, &self).await,
                "chat_mode" => commands::chat_mode::run(&ctx, &command, &self).await,
                _ => {
                    let response: CreateInteractionResponse = CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("Unknown command :("),
                    );
                    command
                        .create_response(&ctx.http, response)
                        .await
                        .map_err(Into::into)
                }
            };
            if let Err(err) = res {
                let response: CreateInteractionResponse = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "Encountered an error while executing command:\n```\n{}\n```",
                            err.to_string()
                        ))
                        .ephemeral(true),
                );
                if let Err(err) = command.create_response(&ctx.http, response).await {
                    println!("Error responding to slash command: {err:?}");
                }
            }
        }
    }

    async fn message(&self, ctx: Context, msg: SerenityMessage) {
        if msg.author.bot {
            return;
        }

        if msg.content == "https://tenor.com/view/no-witnesses-erase-memory-forget-gif-20806865" {
            commands::reset_context::reset_context(&msg.channel_id, &self.db)
                .await
                .expect("Failed to reset context");
            if let Err(err) = msg
                .channel_id
                .send_message(
                    &ctx,
                    CreateMessage::new()
                        .content("Reset context!")
                        .reference_message(&msg),
                )
                .await
            {
                println!("Error sending message: {err:?}");
            }
            return;
        }

        #[allow(deprecated)]
        let is_private = msg.is_private();
        let mentions_me = is_private || msg.mentions_me(&ctx).await.unwrap_or(false);
        let chat_mode = sqlx::query_as::<_, db::Channel>(indoc! {"
            SELECT *
            FROM channels
            WHERE id = $1;
        "})
        .bind(msg.channel_id.get() as i64)
        .fetch_optional(&self.db)
        .await
        .expect("Failed to read channels table")
        .map(|c| c.chat_mode)
        .unwrap_or(db::ChatMode::MentionsOnlyAllContext);

        let mut transaction = self
            .db
            .begin()
            .await
            .expect("Failed to acquire transaction");

        sqlx::query(indoc! {"
                WITH ensured_channel AS (
                    INSERT INTO channels (id)
                    VALUES ($1)
                    ON CONFLICT (id) DO NOTHING
                    RETURNING id
                ),
                locked AS (
                    SELECT id
                    FROM channels
                    WHERE id = $1
                    FOR UPDATE
                )
                INSERT INTO messages (
                    id, is_self, mentions_self, sender, sender_name, sender_display_name, guild, channel, contents, reply
                ) VALUES (
                    $2, false, $3, $4, $5, $6, $7, $1, $8, (SELECT id FROM messages WHERE id = $9)
                );
            "})
            .bind(msg.channel_id.get() as i64)
            .bind(msg.id.get() as i64)
            .bind(mentions_me)
            .bind(msg.author.id.get() as i64)
            .bind(&msg.author.name)
            .bind(msg.author.display_name())
            .bind(msg.guild_id.map(|id| id.get() as i64))
            .bind(msg.content_safe(&ctx))
            .bind(msg.referenced_message.clone().map(|m| m.id.get() as i64))
            .execute(&mut *transaction)
            .await
            .expect("Failed to add message to database");

        if mentions_me || chat_mode == db::ChatMode::FreeResponse {
            if let Err(err) = chatbot::generate(
                &mut transaction,
                &self.openai,
                &self.config,
                &msg.channel_id,
                &msg,
                &ctx,
                mentions_me,
                chat_mode,
            )
            .await
            {
                if let Err(err2) = msg
                    .channel_id
                    .send_message(
                        &ctx,
                        CreateMessage::new()
                            .content(format!("Encountered an error:\n```\n{err:?}\n```"))
                            .allowed_mentions(CreateAllowedMentions::new()),
                    )
                    .await
                {
                    println!("Fatal error: {err:?} {err2:?}");
                }
            };
        }

        transaction
            .commit()
            .await
            .expect("Failed to commit transaction");
    }
}
