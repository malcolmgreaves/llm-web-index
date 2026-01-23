//! Mock LLM provider for testing
//!
//! This module provides a mock implementation of the `LlmProvider` trait
//! that can be configured to return predefined responses or errors,
//! without making real API calls.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::{Error, llms::LlmProvider};

/// Mock LLM provider for testing
///
/// Can be configured to:
/// - Return specific responses based on prompt content
/// - Return a default response for any prompt
/// - Simulate API failures
pub struct MockLlmProvider {
    /// Map of prompt substrings to responses
    /// If the prompt contains the key, return the corresponding response
    responses: HashMap<String, String>,
    /// Default response if no specific match found
    default_response: Option<String>,
    /// If true, always return an error
    should_fail: bool,
}

impl MockLlmProvider {
    /// Create a new empty mock provider
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            default_response: None,
            should_fail: false,
        }
    }

    /// Create a mock that returns a specific response when the prompt contains the given text
    pub fn with_response(prompt_contains: &str, response: &str) -> Self {
        let mut provider = Self::new();
        provider
            .responses
            .insert(prompt_contains.to_string(), response.to_string());
        provider
    }

    /// Create a mock with multiple configured responses
    pub fn with_responses(responses: Vec<(&str, &str)>) -> Self {
        let mut provider = Self::new();
        for (prompt_part, response) in responses {
            provider.responses.insert(prompt_part.to_string(), response.to_string());
        }
        provider
    }

    /// Create a mock with a default response for any prompt
    pub fn with_default(response: &str) -> Self {
        Self {
            responses: HashMap::new(),
            default_response: Some(response.to_string()),
            should_fail: false,
        }
    }

    /// Create a mock that always fails with an error
    pub fn with_failure() -> Self {
        Self {
            responses: HashMap::new(),
            default_response: None,
            should_fail: true,
        }
    }

    /// Create a mock that returns a valid llms.txt file
    pub fn with_valid_llms_txt() -> Self {
        Self::with_default(sample_valid_llms_txt())
    }

    /// Create a mock that returns invalid markdown
    pub fn with_invalid_markdown() -> Self {
        Self::with_default(sample_invalid_markdown())
    }

    /// Create a mock that returns valid markdown but invalid llms.txt format
    pub fn with_invalid_llms_txt() -> Self {
        Self::with_default(sample_invalid_llms_txt())
    }

    /// Add a response mapping to this provider
    pub fn add_response(&mut self, prompt_contains: &str, response: &str) {
        self.responses.insert(prompt_contains.to_string(), response.to_string());
    }

    /// Set the default response
    pub fn set_default(&mut self, response: &str) {
        self.default_response = Some(response.to_string());
    }

    /// Set whether this provider should fail
    pub fn set_should_fail(&mut self, should_fail: bool) {
        self.should_fail = should_fail;
    }
}

impl Default for MockLlmProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete_prompt(&self, prompt: &str) -> Result<String, Error> {
        if self.should_fail {
            // Use InvalidLlmsTxtFormat as a generic error for mock failures
            return Err(Error::InvalidLlmsTxtFormat(
                "Mock LLM provider configured to fail".to_string(),
            ));
        }

        // Try to find a matching response based on prompt content
        for (key, response) in &self.responses {
            if prompt.contains(key) {
                return Ok(response.clone());
            }
        }

        // Use default response if available
        if let Some(default) = &self.default_response {
            return Ok(default.clone());
        }

        // No response configured
        Err(Error::InvalidLlmsTxtFormat(
            "Mock LLM provider has no response configured for this prompt".to_string(),
        ))
    }
}

//
// Test Fixtures
//

/// Sample valid llms.txt content that passes validation
pub fn sample_valid_llms_txt() -> &'static str {
    r#"# Example Website

> A comprehensive example website for testing purposes.

- [Home](https://example.com)
- [About](https://example.com/about)
- [Documentation](https://example.com/docs)

## Details

This is a test website with some detailed information.

- [Contact](https://example.com/contact)
- [API Reference](https://example.com/api)
"#
}

/// Sample HTML content for testing
pub fn sample_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Example Website</title>
</head>
<body>
    <header>
        <h1>Welcome to Example.com</h1>
        <nav>
            <ul>
                <li><a href="/">Home</a></li>
                <li><a href="/about">About</a></li>
                <li><a href="/contact">Contact</a></li>
            </ul>
        </nav>
    </header>
    <main>
        <section>
            <h2>About Us</h2>
            <p>This is a test website for demonstration purposes.</p>
        </section>
        <section>
            <h2>Features</h2>
            <ul>
                <li>Feature 1</li>
                <li>Feature 2</li>
                <li>Feature 3</li>
            </ul>
        </section>
    </main>
    <footer>
        <p>&copy; 2024 Example.com</p>
    </footer>
</body>
</html>
"#
}

/// Sample invalid markdown (unmatched brackets)
pub fn sample_invalid_markdown() -> &'static str {
    r#"# Broken Markdown

This has [unmatched brackets(https://example.com)

- [Also broken](/link
"#
}

/// Sample valid markdown but invalid llms.txt format (missing required structure)
pub fn sample_invalid_llms_txt() -> &'static str {
    r#"# Just A Title

Some text without the required structure.

No links or proper sections.
"#
}

/// Minimal valid llms.txt
pub fn minimal_llms_txt() -> &'static str {
    r#"# Site

> Description

- [Link](/)
"#
}

/// Sample HTML with various edge cases
pub fn sample_complex_html() -> &'static str {
    r#"<!DOCTYPE html>
<html>
<head>
    <title>Complex Example</title>
    <script>
        // This script should be ignored
        console.log("test");
    </script>
    <style>
        body { margin: 0; }
    </style>
</head>
<body>
    <h1>Complex Content</h1>

    <!-- HTML comments -->
    <div class="content">
        <p>Nested <strong>bold <em>italic</em></strong> text.</p>
        <ul>
            <li>Item 1
                <ul>
                    <li>Subitem 1.1</li>
                    <li>Subitem 1.2</li>
                </ul>
            </li>
            <li>Item 2</li>
        </ul>
    </div>

    <table>
        <tr><th>Header 1</th><th>Header 2</th></tr>
        <tr><td>Data 1</td><td>Data 2</td></tr>
    </table>

    <pre><code>Code block content</code></pre>
</body>
</html>
"#
}

/// Sample empty HTML (minimal valid HTML)
pub fn sample_empty_html() -> &'static str {
    r#"<!DOCTYPE html>
<html>
<head><title>Empty</title></head>
<body></body>
</html>
"#
}

/// Sample malformed HTML (missing closing tags)
pub fn sample_malformed_html() -> &'static str {
    r#"<html>
<head>
<title>Malformed
<body>
<h1>Missing closing tags
<p>Unclosed paragraph
<div>Unclosed div
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_with_default_response() {
        let provider = MockLlmProvider::with_default("test response");
        let result = provider.complete_prompt("any prompt").await.unwrap();
        assert_eq!(result, "test response");
    }

    #[tokio::test]
    async fn test_mock_with_specific_response() {
        let provider = MockLlmProvider::with_response("generate", "generated content");

        let result = provider.complete_prompt("generate llms.txt").await.unwrap();
        assert_eq!(result, "generated content");
    }

    #[tokio::test]
    async fn test_mock_with_multiple_responses() {
        let provider =
            MockLlmProvider::with_responses(vec![("generate", "generation response"), ("update", "update response")]);

        assert_eq!(
            provider.complete_prompt("generate content").await.unwrap(),
            "generation response"
        );
        assert_eq!(
            provider.complete_prompt("update content").await.unwrap(),
            "update response"
        );
    }

    #[tokio::test]
    async fn test_mock_with_failure() {
        let provider = MockLlmProvider::with_failure();
        let result = provider.complete_prompt("any prompt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_with_valid_llms_txt() {
        let provider = MockLlmProvider::with_valid_llms_txt();
        let result = provider.complete_prompt("any prompt").await.unwrap();
        assert!(result.contains("# Example Website"));
        assert!(result.contains("[Home]"));
    }

    #[tokio::test]
    async fn test_mock_no_response_configured() {
        let provider = MockLlmProvider::new();
        let result = provider.complete_prompt("any prompt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_add_response() {
        let mut provider = MockLlmProvider::new();
        provider.add_response("test", "test response");

        let result = provider.complete_prompt("test prompt").await.unwrap();
        assert_eq!(result, "test response");
    }

    #[tokio::test]
    async fn test_mock_set_should_fail() {
        let mut provider = MockLlmProvider::with_default("response");
        provider.set_should_fail(true);

        let result = provider.complete_prompt("any prompt").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_sample_valid_llms_txt_contains_title() {
        let content = sample_valid_llms_txt();
        assert!(content.contains("# Example Website"));
    }

    #[test]
    fn test_sample_html_is_valid() {
        let html = sample_html();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn test_fixtures_are_different() {
        assert_ne!(sample_valid_llms_txt(), sample_invalid_llms_txt());
        assert_ne!(sample_html(), sample_complex_html());
        assert_ne!(sample_html(), sample_empty_html());
    }
}
