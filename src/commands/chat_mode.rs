use std::str::FromStr;

use indoc::indoc;
use serenity::all::*;

use crate::{db, handler::Handler};

pub async fn run(
    ctx: &Context,
    command: &CommandInteraction,
    handler: &Handler,
) -> eyre::Result<()> {
    let response = if let Some(ResolvedOption {
        value: ResolvedValue::String(mode),
        ..
    }) = command.data.options().first().as_ref()
    {
        let new_mode = db::ChatMode::from_str(&mode)?;
        sqlx::query(indoc! {"
            INSERT INTO channels (id, chat_mode)
            VALUES ($1, $2)
            ON CONFLICT (id)
            DO UPDATE SET
                chat_mode = $2;
        "})
        .bind(command.channel_id.get() as i64)
        .bind(&new_mode)
        .execute(&handler.db)
        .await?;
        format!("Updated Lumi's chat mode to {}", new_mode.to_string())
    } else {
        let channel: Option<db::Channel> = sqlx::query_as(indoc! {"
            SELECT *
            FROM channels
            WHERE id = $1;
        "})
        .bind(command.channel_id.get() as i64)
        .fetch_optional(&handler.db)
        .await?;
        if let Some(channel) = channel {
            format!(
                "Lumi's current chat mode is {}",
                channel.chat_mode.to_string()
            )
        } else {
            "Lumi does not have a chat mode set for this channel".into()
        }
    };

    let response: CreateInteractionResponse = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(response)
            .ephemeral(false),
    );
    if let Err(err) = command.create_response(&ctx, response).await {
        println!("Error responding to slash command: {err:?}");
    }

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("chat_mode")
        .description("Set or view Lumi's chat mode for the current channel")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "mode",
                "Which chat mode Lumi should use",
            )
            .add_string_choice("Free Response", "free_response")
            .add_string_choice("Mentions Only", "mentions_only")
            .add_string_choice("Mentions Only All Context", "mentions_only_all_context"),
        )
}
