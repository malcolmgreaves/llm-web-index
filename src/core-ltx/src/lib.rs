pub mod llms;
pub mod md_llm_txt;
pub mod web_html;

pub use md_llm_txt::{LlmsTxt, Markdown, is_valid_markdown, validate_is_llm_txt};
pub use web_html::{download_html, is_valid_url};

#[derive(Debug)]
pub enum Error {
    InvalidMarkdown(nom::Err<nom::error::Error<String>>),
    InvalidLlmsTxtFormat(String),
    DownloadError(reqwest::Error),
    InvalidUrl(url::ParseError),
    InaccessibleWebsite(String),
    InvalidHtml(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidMarkdown(err) => write!(f, "Not valid Markdown: {}", err),
            Error::InvalidLlmsTxtFormat(msg) => write!(f, "Not valid llms.txt Format: {}", msg),
            Error::InvalidUrl(url) => write!(f, "Not a valid URL: {}", url),
            Error::InaccessibleWebsite(url) => write!(f, "Cannot reach website: {}", url),
            Error::InvalidHtml(txt) => write!(f, "Not a valid HTML: {}", txt),
            Error::DownloadError(err) => write!(f, "Download error: {}", err),
        }
    }
}

impl std::error::Error for Error {}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::DownloadError(err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::InvalidUrl(err)
    }
}
