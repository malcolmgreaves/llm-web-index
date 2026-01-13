use html5ever::{
    parse_document,
    serialize::{SerializeOpts, serialize},
    tendril::TendrilSink,
};
use markup5ever_rcdom::{RcDom, SerializableHandle};

/// Parses and validates the input as HTML. Returns valid HTML 5 or an error.
/// Attempts to fix the input string according to HTML5 parsing rules.
/// This normalizes the HTML, which helps reduce false positives in change detection.
pub fn parse_html(content: &str) -> Result<String, anyhow::Error> {
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

    #[test]
    fn test_parse_html() {
        let expected = "<html><head></head><body><h1>Hello, World!</h1></body></html>";
        for html in [
            "<html><body><h1>Hello, World!</h1></body></html>", // valid
            "<html><body><h1>Hello, World!</body></html>",      // missing closing </h1>
        ] {
            let parsed_html = parse_html(html).unwrap();
            assert_eq!(parsed_html, expected);
        }
    }
}
