//! HTML parsing utilities for extracting metadata and content.

use scraper::{Html, Selector};

/// Extracts the title from HTML content.
///
/// Looks for the `<title>` element in the HTML head.
///
/// # Examples
///
/// ```
/// # use rule_llms_txt_gen::get_title;
/// let html = r#"<html><head><title>Example Title</title></head></html>"#;
/// assert_eq!(get_title(html), Some("Example Title".to_string()));
/// ```
pub fn get_title(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("head > title").ok()?;

    document
        .select(&selector)
        .next()
        .map(|element| element.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Extracts the description from HTML meta tags.
///
/// Tries the following meta tags in order:
/// 1. `<meta name="description" content="...">`
/// 2. `<meta property="og:description" content="...">`
/// 3. `<meta name="twitter:description" content="...">`
///
/// # Examples
///
/// ```
/// # use rule_llms_txt_gen::get_description;
/// let html = r#"<html><head><meta name="description" content="Example description"></head></html>"#;
/// assert_eq!(get_description(html), Some("Example description".to_string()));
/// ```
pub fn get_description(html: &str) -> Option<String> {
    let document = Html::parse_document(html);

    // Try meta name="description"
    if let Ok(selector) = Selector::parse(r#"head > meta[name="description"]"#) {
        if let Some(element) = document.select(&selector).next() {
            if let Some(content) = element.value().attr("content") {
                if !content.is_empty() {
                    return Some(content.to_string());
                }
            }
        }
    }

    // Try meta property="og:description"
    if let Ok(selector) = Selector::parse(r#"head > meta[property="og:description"]"#) {
        if let Some(element) = document.select(&selector).next() {
            if let Some(content) = element.value().attr("content") {
                if !content.is_empty() {
                    return Some(content.to_string());
                }
            }
        }
    }

    // Try meta name="twitter:description"
    if let Ok(selector) = Selector::parse(r#"head > meta[name="twitter:description"]"#) {
        if let Some(element) = document.select(&selector).next() {
            if let Some(content) = element.value().attr("content") {
                if !content.is_empty() {
                    return Some(content.to_string());
                }
            }
        }
    }

    None
}

/// Extracts the main content area from HTML.
///
/// Tries the following selectors in order:
/// 1. `<main>`
/// 2. `[role=main]`
/// 3. `.content`, `#content`, `.post`, `.docs`, `.article` (first match)
/// 4. `<article>`
/// 5. `<body>`
///
/// If all selectors fail, returns the entire HTML.
///
/// This is used by `gen_full()` to extract the main content for conversion to markdown.
pub fn extract_main_content(html: &str) -> String {
    let document = Html::parse_document(html);

    // Try main element
    if let Ok(selector) = Selector::parse("main") {
        if let Some(element) = document.select(&selector).next() {
            return element.html();
        }
    }

    // Try [role=main]
    if let Ok(selector) = Selector::parse("[role=main]") {
        if let Some(element) = document.select(&selector).next() {
            return element.html();
        }
    }

    // Try common content selectors
    let content_selectors = vec![".content", "#content", ".post", ".docs", ".article"];

    for sel_str in content_selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            if let Some(element) = document.select(&selector).next() {
                return element.html();
            }
        }
    }

    // Try article element
    if let Ok(selector) = Selector::parse("article") {
        if let Some(element) = document.select(&selector).next() {
            return element.html();
        }
    }

    // Try body element
    if let Ok(selector) = Selector::parse("body") {
        if let Some(element) = document.select(&selector).next() {
            return element.html();
        }
    }

    // Fallback to entire HTML
    html.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_title() {
        let html = r#"<html><head><title>Example Title</title></head></html>"#;
        assert_eq!(get_title(html), Some("Example Title".to_string()));

        let html_no_title = r#"<html><head></head></html>"#;
        assert_eq!(get_title(html_no_title), None);

        let html_empty_title = r#"<html><head><title></title></head></html>"#;
        assert_eq!(get_title(html_empty_title), None);
    }

    #[test]
    fn test_get_description() {
        let html = r#"<html><head><meta name="description" content="Example description"></head></html>"#;
        assert_eq!(get_description(html), Some("Example description".to_string()));

        let html_og = r#"<html><head><meta property="og:description" content="OG description"></head></html>"#;
        assert_eq!(get_description(html_og), Some("OG description".to_string()));

        let html_twitter =
            r#"<html><head><meta name="twitter:description" content="Twitter description"></head></html>"#;
        assert_eq!(get_description(html_twitter), Some("Twitter description".to_string()));

        let html_no_desc = r#"<html><head></head></html>"#;
        assert_eq!(get_description(html_no_desc), None);
    }

    #[test]
    fn test_get_description_priority() {
        // Should prefer meta description over og:description
        let html = r#"
            <html>
                <head>
                    <meta name="description" content="Meta description">
                    <meta property="og:description" content="OG description">
                </head>
            </html>
        "#;
        assert_eq!(get_description(html), Some("Meta description".to_string()));
    }

    #[test]
    fn test_extract_main_content() {
        let html = r#"<html><body><main><p>Main content</p></main></body></html>"#;
        let content = extract_main_content(html);
        assert!(content.contains("Main content"));

        let html_article = r#"<html><body><article><p>Article content</p></article></body></html>"#;
        let content = extract_main_content(html_article);
        assert!(content.contains("Article content"));
    }
}
