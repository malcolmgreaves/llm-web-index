//! Error types for the llms.txt generation library.

use thiserror::Error;

/// Main error type for llms.txt generation operations.
#[derive(Debug, Error)]
pub enum LlmsGenError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Invalid URL format
    #[error("Invalid URL: {0}")]
    UrlParseError(#[from] url::ParseError),

    /// Sitemap parsing failed
    #[error("Sitemap parsing failed: {0}")]
    SitemapError(String),

    /// Invalid substitution command format
    #[error("Invalid substitution command: {0}")]
    InvalidSubstitution(String),

    /// HTML parsing error
    #[error("HTML parsing error: {0}")]
    HtmlParseError(String),

    /// Regex error
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    /// Glob pattern error
    #[error("Glob pattern error: {0}")]
    GlobError(#[from] globset::Error),
}

/// Type alias for Result with LlmsGenError
pub type Result<T> = std::result::Result<T, LlmsGenError>;
