pub mod llms;

#[derive(Debug)]
pub struct Error;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<error>")
    }
}

impl std::error::Error for Error {}

pub struct Markdown(String);

impl Markdown {
    pub fn new(maybe_markdown: String) -> Result<Self, Error> {
        if Self::is_markdown(&maybe_markdown) {
            Ok(Markdown(maybe_markdown))
        } else {
            Err(Error)
        }
    }

    pub fn is_markdown(content: &str) -> bool {
        unimplemented!("Need to implement markdown validation, got: '{}'", content)
    }
}

pub struct LlmTxt {
    pub file: Markdown,
}

impl LlmTxt {
    fn new(llm_txt: Markdown) -> Self {
        LlmTxt { file: llm_txt }
    }
}
