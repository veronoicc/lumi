use serenity::all::*;

use crate::{Config, handler::Handler};

pub async fn run(
    ctx: &Context,
    command: &CommandInteraction,
    handler: &Handler,
) -> eyre::Result<()> {
    let config: Config = toml::from_str(
        &tokio::fs::read_to_string("config.toml")
            .await
            .expect("Failed to read config file"),
    )
    .expect("Failed to parse config file");
    *handler.config.write().await = config;

    let response: CreateInteractionResponse = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("Reloaded config!")
            .ephemeral(true),
    );
    if let Err(err) = command.create_response(&ctx, response).await {
        println!("Error responding to slash command: {err:?}");
    }

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("reload").description("Reload Lumi's config file")
}
