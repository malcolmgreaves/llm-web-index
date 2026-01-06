use async_openai::{
    Client,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
};

#[derive(Debug)]
pub enum ChatGptError {
    ApiError(async_openai::error::OpenAIError),
    NoResponse,
}

impl std::fmt::Display for ChatGptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatGptError::ApiError(e) => write!(f, "OpenAI API error: {}", e),
            ChatGptError::NoResponse => write!(f, "No response from ChatGPT"),
        }
    }
}

impl std::error::Error for ChatGptError {}

impl From<async_openai::error::OpenAIError> for ChatGptError {
    fn from(err: async_openai::error::OpenAIError) -> Self {
        ChatGptError::ApiError(err)
    }
}

/// Sends a simple prompt to ChatGPT and returns the response.
///
/// This function uses the OpenAI API to send a static prompt "Tell me a one-liner joke."
/// and returns the response text.
///
/// # Errors
///
/// Returns `ChatGptError` if:
/// - The OpenAI API call fails
/// - No response is received from the API
///
/// # Environment Variables
///
/// Requires `OPENAI_API_KEY` to be set in the environment.
pub async fn send_simple_prompt() -> Result<String, ChatGptError> {
    let client = Client::new();

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-3.5-turbo")
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are a helpful assistant.")
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content("Tell me a one-liner joke.")
                .build()?
                .into(),
        ])
        .build()?;

    let response = client.chat().create(request).await?;

    response
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .ok_or(ChatGptError::NoResponse)
}

#[cfg(test)]
mod tests {
    use super::*;

    use common_ltx::is_env_set;

    #[tokio::test]
    async fn test_send_simple_prompt() {
        if is_env_set("OPENAI_API_KEY") {
            let result = send_simple_prompt().await;

            match result {
                Ok(response) => {
                    println!("ChatGPT response: {}", response);
                    assert!(!response.is_empty(), "Response should not be empty");
                }
                Err(ChatGptError::ApiError(e)) => {
                    panic!("API error: {}", e);
                }
                Err(ChatGptError::NoResponse) => {
                    panic!("Unexpected NoResponse error");
                }
            }
        } else {
            println!("[SKIP] OPENAI_API_KEY is not set");
        }
    }
}
