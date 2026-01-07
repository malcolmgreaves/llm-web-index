pub mod chatgpt;
pub mod claude;
pub mod prompts;

use async_trait::async_trait;
pub use prompts::{
    prompt_generate_llms_txt, prompt_retry_generate_llms_txt, prompt_retry_update_llms_txt, prompt_update_llms_txt,
};

pub use chatgpt::ChatGpt;

use crate::{Error, LlmsTxt, download, is_valid_markdown, is_valid_url, validate_is_llm_txt};

/// Interface to a hosted LLM that lets us complete a prompt and await a response.
#[async_trait]
pub trait LlmProvider {
    async fn complete_prompt(&self, prompt: &str) -> Result<String, Error>;
}

/// Downloads a website's HTML and generates an llms.txt file for it using an LLM.
pub async fn generate_llms_txt_url<P: LlmProvider>(provider: &P, website_url: &str) -> Result<LlmsTxt, Error> {
    let url = is_valid_url(website_url)?;
    let html = download(&url).await?;
    generate_llms_txt(provider, &html).await
}

/// Generates an llms.txt file from a website's HTML using an LLM provider with specific prompting.
pub async fn generate_llms_txt<P: LlmProvider>(provider: &P, html: &str) -> Result<LlmsTxt, Error> {
    let prompt = prompt_generate_llms_txt(html)?;
    let llm_response = provider.complete_prompt(&prompt).await?;

    match is_valid_markdown(&llm_response) {
        Ok(markdown) => match validate_is_llm_txt(markdown) {
            Ok(llms_txt) => Ok(llms_txt),
            Err(e) => retry_generate(provider, &html, &llm_response, &e).await,
        },
        Err(e) => retry_generate(provider, &html, &llm_response, &e).await,
    }
}

/// Updates an old llms.txt file with the newly downloaded website changes.
pub async fn update_llms_txt_url<P: LlmProvider>(
    provider: &P,
    existing_llms_txt: &str,
    website_url: &str,
) -> Result<LlmsTxt, Error> {
    let url = is_valid_url(website_url)?;
    let html = download(&url).await?;
    update_llms_txt(provider, existing_llms_txt, &html).await
}

/// Updates an old llms.txt file with the website's new content.
pub async fn update_llms_txt<P: LlmProvider>(
    provider: &P,
    existing_llms_txt: &str,
    html: &str,
) -> Result<LlmsTxt, Error> {
    validate_is_llm_txt(is_valid_markdown(existing_llms_txt)?)?;

    let prompt = prompt_update_llms_txt(existing_llms_txt, &html)?;
    let llm_response = provider.complete_prompt(&prompt).await?;

    match is_valid_markdown(&llm_response) {
        Ok(markdown) => match validate_is_llm_txt(markdown) {
            Ok(llms_txt) => Ok(llms_txt),
            Err(e) => retry_update(provider, existing_llms_txt, &html, &llm_response, &e).await,
        },
        Err(e) => retry_update(provider, existing_llms_txt, &html, &llm_response, &e).await,
    }
}

async fn retry_generate<P: LlmProvider>(
    provider: &P,
    html: &str,
    llm_response: &str,
    error: &Error,
) -> Result<LlmsTxt, Error> {
    retry(
        provider,
        &prompt_retry_generate_llms_txt(html, llm_response, &error.to_string())?,
    )
    .await
}

async fn retry_update<P: LlmProvider>(
    provider: &P,
    existing_llms_txt: &str,
    html: &str,
    llm_response: &str,
    error: &Error,
) -> Result<LlmsTxt, Error> {
    retry(
        provider,
        &prompt_retry_update_llms_txt(existing_llms_txt, html, llm_response, &error.to_string())?,
    )
    .await
}

async fn retry<P: LlmProvider>(provider: &P, prompt: &str) -> Result<LlmsTxt, Error> {
    let new_llm_response = provider.complete_prompt(prompt).await?;
    is_valid_markdown(&new_llm_response).and_then(validate_is_llm_txt)
}
