use url::Url;

use html5ever::{
    parse_document,
    serialize::{SerializeOpts, serialize},
    tendril::TendrilSink,
};
use markup5ever_rcdom::{RcDom, SerializableHandle};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_download() {
        let url = Url::parse("https://example.com").unwrap();
        let content = download(&url).await.unwrap();
        assert!(!content.is_empty());
    }

    #[tokio::test]
    async fn test_parse_html() {
        let expected = "<html><head></head><body><h1>Hello, World!</h1></body></html>";
        for html in [
            "<html><body><h1>Hello, World!</h1></body></html>", // valid
            "<html><body><h1>Hello, World!</body></html>", // assert that it can close missing tags -- this is missing a closing </h1>
        ] {
            let parsed_html = parse_html(html).unwrap();
            assert_eq!(parsed_html, expected);
        }
    }
}
