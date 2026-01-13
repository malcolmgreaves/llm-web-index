use crate::md_llm_txt::LlmsTxt;

/// Interface for generating an llms.txt file from a website's HTML content.
pub trait LlmsTxtGenerator {
    fn generate_llms_txt(&self, html: &str) -> Result<LlmsTxt, GenError>;
}

/// Interface for updating an existing llms.txt file with new content from a website's HTML.
pub trait LlmsTxtUpdater {
    fn update_llms_txt(&self, prior_llms_txt: &LlmsTxt, html: &str) -> Result<LlmsTxt, GenError>;
}

/// When something goes wrong during the llms.txt generation (or update) process.
#[derive(Debug)]
pub enum GenError {
    Error(String),
}

impl std::error::Error for GenError {}

impl std::fmt::Display for GenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GenError::Error(msg) => write!(f, "Generation error: {}", msg),
        }
    }
}
