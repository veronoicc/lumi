use openai_api_rs::v1::{api::OpenAIClientBuilder, chat_completion::Reasoning};
use serde::Deserialize;
use serenity::all::*;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::{Mutex, RwLock};

use crate::handler::Handler;

pub mod chat;
pub mod commands;
pub mod db;
pub mod handler;

#[derive(Deserialize)]
pub struct Config {
    pub discord: ConfigDiscord,
    pub openrouter: ConfigOpenrouter,
    pub database: ConfigDatabase,
}

#[derive(Deserialize)]
pub struct ConfigDiscord {
    pub bot_token: String,
}

#[derive(Deserialize)]
pub struct ConfigOpenrouter {
    pub api_key: String,
    pub chat: ConfigModel,
    pub social: ConfigModel,
    pub window_threshold: usize,
    pub max_attempts: isize,
}

#[derive(Deserialize)]
pub struct ConfigModel {
    pub model: String,
    pub reasoning: Option<Reasoning>,
}

#[derive(Deserialize)]
pub struct ConfigDatabase {
    pub url: String,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let config: Config = toml::from_str(
        &tokio::fs::read_to_string("config.toml")
            .await
            .expect("Failed to read config file"),
    )
    .expect("Failed to parse config file");
    let openai = OpenAIClientBuilder::new()
        .with_api_key(&config.openrouter.api_key)
        .with_endpoint("https://openrouter.ai/api/v1/")
        .build()
        .expect("Failed to build OpenAI client");
    let bot_token = config.discord.bot_token.clone();
    let db = PgPoolOptions::new()
        .connect(&config.database.url)
        .await
        .expect("Failed to connect to postgres database");
    for stmt in include_str!("../setup.sql").split("-- break") {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            sqlx::query(stmt)
                .execute(&db)
                .await
                .expect("Database setup script failed");
        }
    }

    let handler = Handler {
        config: RwLock::new(config),
        openai: Mutex::new(openai),
        db,
    };

    let mut discord = Client::builder(
        bot_token,
        GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MEMBERS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::GUILD_MESSAGE_REACTIONS
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::DIRECT_MESSAGE_REACTIONS,
    )
    .event_handler(handler)
    .await
    .expect("Err creating client");

    if let Err(err) = discord.start().await {
        println!("Client error: {err:?}");
    }

    Ok(())
}
