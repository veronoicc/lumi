use indoc::indoc;
use serenity::all::*;
use sqlx::PgPool;

use crate::handler::Handler;

pub async fn run(
    ctx: &Context,
    command: &CommandInteraction,
    handler: &Handler,
) -> eyre::Result<()> {
    reset_context(&command.channel_id, &handler.db).await?;
    let response: CreateInteractionResponse = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("Reset context!")
            .ephemeral(false),
    );
    if let Err(err) = command.create_response(&ctx, response).await {
        println!("Error responding to slash command: {err:?}");
    }

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("reset_context").description("Reset Lumi's context for the current channel")
}

pub async fn reset_context(channel_id: &ChannelId, db: &PgPool) -> eyre::Result<()> {
    sqlx::query(indoc! {"
        UPDATE channels
        SET context_window = extract(epoch FROM now())::bigint
        WHERE id = $1;
    "})
    .bind(channel_id.get() as i64)
    .execute(db)
    .await?;
    Ok(())
}
