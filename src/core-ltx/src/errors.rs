/// Custom error type for website downloading & llms.txt format validation.
#[derive(Debug)]
pub enum Error {
    /// Cannot download website because user supplied an invalid URL.
    InvalidUrl(url::ParseError),

    /// Website download failed.
    DownloadError(reqwest::Error),

    /// HTML is invalid, even after attempting to fix using HTML5 rules.
    InvalidHtml(String),

    /// File is not valid markdown.
    InvalidMarkdown(nom::Err<nom::error::Error<String>>),

    /// Markdown file does not adhere to the llms.txt format.
    InvalidLlmsTxtFormat(String),

    /// Internal error: prompt substitution failed.
    PromptCreationFailure(subst::Error),

    /// Error calling ChatGPT
    ChatGptError(async_openai::error::OpenAIError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidUrl(url) => write!(f, "Not a valid URL: {}", url),
            Error::DownloadError(err) => write!(f, "Download error: {}", err),
            Error::InvalidHtml(txt) => write!(f, "Not a valid HTML: {}", txt),
            Error::InvalidMarkdown(err) => write!(f, "Not valid Markdown: {}", err),
            Error::InvalidLlmsTxtFormat(msg) => write!(f, "Not valid llms.txt Format: {}", msg),
            Error::PromptCreationFailure(err) => write!(f, "Failed to create prompt: {}", err),
            Error::ChatGptError(err) => write!(f, "Error calling ChatGPT: {}", err),
        }
    }
}

impl std::error::Error for Error {}

/// Request errors occur during the download process.
impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::DownloadError(err)
    }
}

/// URL parsing errors occur during the URL validation process.
impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::InvalidUrl(err)
    }
}

/// Converting from bytes to UTF-8 strings occurs in the HTML parsing & validation process.
impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Error::InvalidHtml(err.to_string())
    }
}

/// io Errors occur during the HTML parsing & validation process.
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::InvalidHtml(err.to_string())
    }
}

impl From<subst::Error> for Error {
    fn from(err: subst::Error) -> Self {
        Error::PromptCreationFailure(err)
    }
}

impl From<async_openai::error::OpenAIError> for Error {
    fn from(err: async_openai::error::OpenAIError) -> Self {
        Error::ChatGptError(err)
    }
}
