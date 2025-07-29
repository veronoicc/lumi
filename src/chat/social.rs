use openai_api_rs::v1::{
    api::OpenAIClient,
    chat_completion::{
        ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse, Content, MessageRole,
    },
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::Config;

#[derive(Serialize, Deserialize)]
pub struct ShouldReply {
    pub should_reply: bool,
}

pub async fn should_reply(
    mut context: Vec<ChatCompletionMessage>,
    openai: &Mutex<OpenAIClient>,
    config: &RwLock<Config>,
) -> eyre::Result<bool> {
    let mut i = config.read().await.openrouter.max_attempts;
    loop {
        let response = generate_completion(context.clone(), openai, config).await?;
        let response = &response.choices.first().unwrap().message;
        let result =
            serde_json::from_str::<ShouldReply>(&response.content.clone().unwrap_or_default());
        match result {
            Ok(result) => return Ok(result.should_reply),
            Err(err) => {
                i -= 1;
                if i <= 0 {
                    return Err(err.into());
                }
                context.push(ChatCompletionMessage {
                    role: MessageRole::system,
                    content: Content::Text(format!("Failed to parse JSON:\n{err:?}\nTry again, and ensure your response is valid JSON")),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
        }
    }
}

async fn generate_completion(
    context: Vec<ChatCompletionMessage>,
    openai: &Mutex<OpenAIClient>,
    config: &RwLock<Config>,
) -> eyre::Result<ChatCompletionResponse> {
    let body = {
        let config = &config.read().await.openrouter.social;
        ChatCompletionRequest {
            model: config.model.to_owned(),
            max_tokens: None,
            temperature: Some(0.0),
            top_p: None,
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
