use eyre::bail;
use indoc::indoc;
use serenity::all::*;

use crate::{db, handler::Handler};

pub async fn run(
    ctx: &Context,
    command: &CommandInteraction,
    handler: &Handler,
) -> eyre::Result<()> {
    let response = if let Some(ResolvedOption {
        value: ResolvedValue::String(prompt),
        ..
    }) = command.data.options().first().as_ref()
    {
        let prompt: Option<db::SystemPrompt> = sqlx::query_as(indoc! {"
            SELECT *
            FROM system_prompts
            WHERE name = $1;
        "})
        .bind(prompt)
        .fetch_optional(&handler.db)
        .await?;
        let Some(prompt) = prompt else {
            bail!("Could not find a system prompt for the given name");
        };
        sqlx::query(indoc! {"
            UPDATE channels
            SET system_prompt = $1
            WHERE id = $2;
        "})
        .bind(prompt.id)
        .bind(command.channel_id.get() as i64)
        .execute(&handler.db)
        .await?;
        format!("Set Lumi's system prompt to '`{}`'", prompt.name)
    } else {
        let current_prompt: Option<db::SystemPrompt> = sqlx::query_as(indoc! {"
            SELECT sp.*
            FROM channels ch
            JOIN system_prompts sp ON ch.system_prompt = sp.id
            WHERE ch.id = $1;
        "})
        .bind(command.channel_id.get() as i64)
        .fetch_optional(&handler.db)
        .await?;
        if let Some(current_prompt) = current_prompt {
            format!(
                "Lumi's current system prompt is '`{}`'",
                current_prompt.name
            )
        } else {
            "Lumi does not have a system prompt set for this channel".into()
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
    CreateCommand::new("system_prompt")
        .description("View or change Lumi's system prompt in the current channel")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "prompt",
                "The system prompt for Lumi to use",
            )
            .required(false),
        )
}
