pub mod llms;
pub mod markdown;

pub fn is_valid_markdown(content: &str) -> bool {
    unimplemented!("Need to implement markdown validation, got: '{}'", content);
}

pub fn is_valid_llm_txt(content: &Markdown) -> bool {
    unimplemented!("Need to implement LLM TXT validation, got: '{}'", content);
}

#[derive(Debug)]
pub enum Error {
    InvalidMarkdown,
    InvalidLlmTxtFormat,
    Unknown(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<error>")
    }
}

impl std::error::Error for Error {}

/// Construct a new zero-zied type wrapper around a to-be-validated value.
/// The newtype only exists for values that are valid according to the $is_valid function.
macro_rules! newtype_valid {
    ($name:ident, $inner:path, $is_valid:expr, $error:path, $new_error:expr) => {
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
        pub struct $name($inner);

        impl std::fmt::Display for $name {
            /// Displays $inner only.
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl $name {
            /// Create a new instance of $name if the $inner value is valid. Returns an error on failure.
            pub fn new(maybe_valid_inner: $inner) -> Result<Self, $error> {
                if Self::is_valid(&maybe_valid_inner) {
                    Ok($name(maybe_valid_inner))
                } else {
                    let e: $error = $new_error(&maybe_valid_inner);
                    Err(e)
                }
            }

            /// True if the $inner value is a valid instance of $name. False otherwise.
            pub fn is_valid(maybe_valid_inner: &$inner) -> bool {
                $is_valid(maybe_valid_inner)
            }

            /// Apply a function to transform the $inner value into a new instance.
            /// If the new instance isn't valid, then this results in an instance of `Err($error)`.
            /// Otherwise, it's an `Ok` of the new $inner value wrapped as a $name.
            pub fn map<F>(&self, f: F) -> Result<$name, $error>
            where
                F: FnOnce(&$inner) -> Result<$inner, $error>,
            {
                f(&self.0).map($name)
            }

            /// Destroys the $name wrapper, obtaining the $inner value directly.
            pub fn extract(self) -> $inner {
                self.0
            }
        }
    };
}

newtype_valid!(Markdown, String, is_valid_markdown, Error, |_| {
    Error::InvalidMarkdown
});

newtype_valid!(LlmTxt, Markdown, is_valid_llm_txt, Error, |_| {
    Error::InvalidLlmTxtFormat
});
