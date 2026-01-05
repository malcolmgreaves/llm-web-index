use markdown_ppp::parser::{MarkdownParserState, parse_markdown};
use markdown_ppp::printer::{config::Config, render_markdown};

pub fn example() {
    let input = "# Title\n\nSome *markdown* text with an [example](http://example.com).";
    let state = MarkdownParserState::default();

    match parse_markdown(state, input) {
        Ok(document) => {
            println!("Parsed successfully!");
            // You could inspect `document` here (e.g., ensure first block is a Heading).
            let output = render_markdown(&document, Config::default());
            println!("Reconstructed Markdown:\n{}", output);
        }
        Err(err) => {
            eprintln!("Markdown parse error: {:?}", err);
        }
    }
}
