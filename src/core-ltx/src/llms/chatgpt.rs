use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
        CreateCompletionRequestArgs,
    },
};

use crate::{Error, llms::LlmProvider};

#[derive(Debug, Clone)]
struct ChatGpt {
    pub client: Client<OpenAIConfig>,
    pub model_name: String,
}

impl Default for ChatGpt {
    fn default() -> Self {
        Self {
            client: Client::new(),
            model_name: "gpt-3.5-turbo".to_string(),
        }
    }
}

impl LlmProvider for ChatGpt {
    async fn complete_prompt(&self, prompt: &str) -> Result<String, Error> {
        let request = CreateCompletionRequestArgs::default()
            .model(self.model_name.as_str())
            .prompt(prompt)
            .build()?;

        let response = self.client.completions().create(request).await?;

        let llm_text_response = response
            .choices
            .first()
            .map(|choice| choice.text.clone())
            .unwrap_or("".to_string());

        Ok(llm_text_response)
    }
}
