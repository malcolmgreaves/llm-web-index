use std::iter::FlatMap;

use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessage, ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage, CreateChatCompletionRequestArgs,
    },
};
use async_trait::async_trait;

use crate::{Error, llms::LlmProvider};

#[derive(Debug, Clone)]
pub struct ChatGpt {
    pub client: Client<OpenAIConfig>,
    pub model_name: String,
}

impl Default for ChatGpt {
    fn default() -> Self {
        Self {
            client: Client::new(),
            model_name: "gpt-5-mini".to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for ChatGpt {
    async fn complete_prompt(&self, prompt: &str) -> Result<String, Error> {
        let request = CreateChatCompletionRequestArgs::default()
            // .max_tokens(512u32)
            .model(&self.model_name)
            .messages([
                // Can also use ChatCompletionRequest<Role>MessageArgs for builder pattern
                ChatCompletionRequestSystemMessage::from("You are a helpful assistant. You produce summaries of websites formatted in Markdown according to the llms.txt specification.").into(),
                ChatCompletionRequestUserMessage::from(prompt).into(),
            ])
            .build()?;

        let response = self.client.chat().create(request).await?;

        let llm_text_response = response
            .choices
            .iter()
            .flat_map(|choice| choice.message.content.clone())
            .take(1)
            .fold("".to_string(), |_, item| item);

        Ok(llm_text_response)
    }
}

// struct FirstFlatMap<S, U, F>(FlatMap<S, U, F>)
// where S:Iterator, U:IntoIterator;

// impl <S,U,F> FirstFlatMap<S,U,F>
// where S:Iterator, U:IntoIterator {

//   pub fn first(self) -> Option<U> {

//     let mut maybe_first: Option<U> = None;
//     for x in self.0.take(1) {
//       maybe_first = Some(x);
//     }
//     maybe_first
//   }

// }

// impl <S,U,F> From<FlatMap<S, U, F>> for FirstFlatMap<S,U,F>
// where S:Iterator, U:IntoIterator {
//   fn from(flat_map: FlatMap<S, U, F>) -> Self {
//     FirstFlatMap(flat_map)
//   }
// }
