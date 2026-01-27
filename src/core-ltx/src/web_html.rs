use url::Url;

use html5ever::{
    parse_document,
    serialize::{SerializeOpts, serialize},
    tendril::TendrilSink,
};
use markup5ever_rcdom::{RcDom, SerializableHandle};
use minify_html::{Cfg, minify};

use crate::Error;

/// Validates that the input string is a URL.
pub fn is_valid_url(url: &str) -> Result<Url, Error> {
    let valid_url = Url::parse(url)?;
    Ok(valid_url)
}

/// Downloads the website's content as text.
pub async fn download(url: &Url) -> Result<String, Error> {
    let response = reqwest::get(url.as_str()).await?;
    let text_body = response.text().await?;
    Ok(text_body)
}

/// Parses and validates the input as HTML. Returns valid HTML 5 or an error.
/// Attempts to fix the input string according to HTML5 parsing rules.
pub fn parse_html(content: &str) -> Result<String, Error> {
    let dom: RcDom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut content.as_bytes())?;

    let document: SerializableHandle = dom.document.clone().into();

    let output = {
        let mut output: Vec<u8> = Vec::new();
        serialize(&mut output, &document, SerializeOpts::default())?;
        output
    };

    let html = String::from_utf8(output)?;
    Ok(html)
}

/// Configuration used by `clean_html`.
const CLEAN_HTML_CFG: Cfg = Cfg {
    // Preserve document structure
    keep_closing_tags: true,
    keep_html_and_head_opening_tags: true,
    keep_input_type_text_attr: true,

    // Remove non-semantic content
    keep_comments: false,
    keep_ssi_comments: false,

    // Don't transform embedded code
    minify_css: false,
    minify_js: false,

    // Stay spec-compliant
    minify_doctype: false,
    allow_noncompliant_unquoted_attribute_values: false,
    allow_optimal_entities: false,
    allow_removing_spaces_between_attributes: false,

    // Remove processing instructions and bangs (non-semantic)
    remove_bangs: true,
    remove_processing_instructions: true,

    // Template syntax (not relevant for plain HTML)
    preserve_brace_template_syntax: false,
    preserve_chevron_percent_template_syntax: false,
};

/// Cleans HTML by removing insignificant whitespace while preserving semantics.
///
/// This function:
/// - Collapses whitespace in content areas
/// - Preserves whitespace in `<pre>`, `<code>`, `<textarea>` elements
/// - Keeps all closing tags for structural integrity
/// - Does NOT transform CSS or JS content
/// - Produces spec-compliant output
pub fn clean_html(html: &str) -> Result<String, std::string::FromUtf8Error> {
    let minified = minify(html.as_bytes(), &CLEAN_HTML_CFG);
    String::from_utf8(minified)
}

/// Normalize the HTML and compute and MD5 checksum on the content.
pub fn compute_html_checksum(html: &str) -> Result<String, Error> {
    let normalized = parse_html(html)?;
    let cleaned = clean_html(&normalized)?;
    let digest = md5::compute(cleaned.as_bytes());
    Ok(format!("{:x}", digest))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HTML_EXAMPLES: [&str; 2] = [
        "<html><body><h1>Hello, World!</h1></body></html>", // valid
        "<html><body><h1>Hello, World!</body></html>", // assert that it can close missing tags -- this is missing a closing </h1>
    ];

    #[test]
    fn test_url() {
        let url = "https://example.com";
        assert!(is_valid_url(url).is_ok());

        let url = "invalid";
        assert!(is_valid_url(url).is_err());
    }

    #[tokio::test]
    async fn test_download() {
        let url = Url::parse("https://example.com").unwrap();
        let content = download(&url).await.unwrap();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_parse_html() {
        let expected = "<html><head></head><body><h1>Hello, World!</h1></body></html>";
        for html in HTML_EXAMPLES {
            let parsed_html = parse_html(html).unwrap();
            assert_eq!(parsed_html, expected);
        }
    }

    #[test]
    fn test_compute_html_checksum() {
        let expected = "b5e56c5effa9b4e92f1b5b6f80a5a781";
        for html in HTML_EXAMPLES {
            let checksum = compute_html_checksum(&html).unwrap();
            assert_eq!(checksum, expected);
        }
    }

    #[test]
    fn test_clean_html_removes_whitespace() {
        let input = "<html>  <head>  </head>  <body>  <p>  Hello,   world!  </p>  </body>  </html>";
        let cleaned = clean_html(input).unwrap();
        // Whitespace between tags and within text is collapsed
        assert!(!cleaned.contains("  "));
    }

    #[test]
    fn test_clean_html_preserves_pre_whitespace() {
        let input = "<pre>  code with   spaces  </pre>";
        let cleaned = clean_html(input).unwrap();
        // Whitespace in <pre> is preserved
        assert!(cleaned.contains("  code with   spaces  "));
    }

    #[test]
    fn test_clean_html_removes_comments() {
        let input = "<p>Hello<!-- comment -->World</p>";
        let cleaned = clean_html(input).unwrap();
        assert!(!cleaned.contains("comment"));
        assert!(cleaned.contains("HelloWorld") || cleaned.contains("Hello World"));
    }

    #[test]
    fn test_clean_html_keeps_closing_tags() {
        let input = "<div><p>Text</p></div>";
        let cleaned = clean_html(input).unwrap();
        assert!(cleaned.contains("</p>"));
        assert!(cleaned.contains("</div>"));
    }
}
