use markdown_ppp::ast::{self, GitHubAlertType};
use markdown_ppp::parser::{MarkdownParserState, parse_markdown};
use markdown_ppp::printer::{config::Config, render_markdown};

pub fn is_valid_markdown(content: &str) -> bool {
    match parse_markdown(MarkdownParserState::default(), content) {
        Ok(document) => {
            println!("parsed! document: {:?}", document);
            true
        }
        Err(error) => {
            println!("failed! error: {}", error);
            false
        }
    }
}

pub fn is_valid_llm_txt(content: &Markdown) -> bool {
    unimplemented!("Need to implement LLM TXT validation, got: '{:?}'", content);
}

#[derive(Debug)]
pub enum Error {
    InvalidMarkdown(nom::Err<nom::error::Error<&'static str>>),
    InvalidLlmsTxtFormat(String),
    Unknown(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidMarkdown(err) => write!(f, "Not valid Markdown: {}", err),
            Error::InvalidLlmsTxtFormat(msg) => write!(f, "Not valid llms.txt Format: {}", msg),
            Error::Unknown(msg) => write!(f, "Unknown Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

type Markdown = ast::Document;

#[derive(Debug, Clone)]
pub struct LlmTxt(Markdown);

pub fn validate_is_llm_txt(doc: Markdown) -> Result<LlmTxt, Error> {
    use ast::Block::*;

    #[derive(PartialEq, Eq, Copy, Clone)]
    enum Stage {
        LookingForH1,
        LookingForSummaryBlockquote,
        LookingForOptionalDetails,
        LookingForFileListSections,
    }

    impl std::fmt::Display for Stage {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                Stage::LookingForH1 => write!(f, "Looking for H1"),
                Stage::LookingForSummaryBlockquote => write!(f, "Looking for Summary Blockquote"),
                Stage::LookingForOptionalDetails => {
                    write!(f, "Looking for Optional Detail Section(s)")
                }
                Stage::LookingForFileListSections => {
                    write!(f, "Looking for Optional File List Section(s)")
                }
            }
        }
    }

    /// S == State
    /// We treat validating the markdown file as a sort of abstract state machine.
    /// We walk through the markdown document's abstract syntax tree (AST) and in so doing
    /// validate or invalidate this markdown document as adhereing to the llms.txt format.
    struct S {
        /// Index in the current list of blocks.
        i: usize,
        /// Where the state machine is at.
        stage: Stage,
        /// The name of the website needs to be the first thing -- the H1 header (aka title). Strict requirement.
        has_h1_name_site: bool,
        /// Need a blockquote summarizing the content. Will treat as required.
        has_summary_blockquote: bool,
        /// Optional. If present, then there can be several non-header elements after the blockquote.
        valid_optional_details: bool,
        /// True means they have already been seen. False means that they have not been seen.
        saw_optional_details: bool,
        /// Optional. If present, then each next section has to be an H2 and be a list of URLs / files
        valid_file_list_sections: bool,
    }

    type Step = Result<(), Error>;

    impl S {
        fn initial() -> Self {
            Self {
                i: 0,
                stage: Stage::LookingForH1,
                has_h1_name_site: false,
                has_summary_blockquote: false,
                valid_optional_details: true,
                saw_optional_details: false,
                valid_file_list_sections: true,
            }
        }

        fn final_validation(&self) -> Step {
            if !self.has_h1_name_site {
                return Err(Error::InvalidLlmsTxtFormat("Missing required H1.".into()));
            }
            if !self.has_summary_blockquote {
                return Err(Error::InvalidLlmsTxtFormat(
                    "Missing required summary blockquote.".into(),
                ));
            }
            if !self.valid_optional_details {
                return Err(Error::InvalidLlmsTxtFormat(
                    "Invalid optional details section.".into(),
                ));
            }
            if !self.valid_file_list_sections {
                return Err(Error::InvalidLlmsTxtFormat(
                    "Invalid optional file list sections.".into(),
                ));
            }
            Ok(())
        }

        fn accept_h1(&mut self, content: &[ast::Inline]) -> Step {
            // validate if this is our H1
            if self.stage != Stage::LookingForH1 || self.has_h1_name_site {
                return Err(Error::InvalidLlmsTxtFormat(format!(
                    "H1 already exists. Invalid to have a second H1: '{:?}'",
                    content
                )));
            }

            if self.i != 0 {
                return Err(Error::InvalidLlmsTxtFormat(format!(
                    "H1 must be the first block in the document. Found valid H1 '{:?}' but it was block index {}",
                    content, self.i
                )));
            }

            // it's the first block and it's an H1
            self.has_h1_name_site = true;
            self.stage = Stage::LookingForSummaryBlockquote;
            Ok(())
        }

        fn accept_other_header(&mut self) -> Step {
            match self.stage {
                Stage::LookingForFileListSections | Stage::LookingForOptionalDetails => {
                    // accept: make sure we stay in the file list stage (we could skip over the optional details)
                    self.stage = Stage::LookingForFileListSections;
                    Ok(())
                }
                wrong_stage => {
                    return Err(Error::InvalidLlmsTxtFormat(format!(
                        "Found a header when we were not looking for file lists! We are looking for: {}",
                        wrong_stage
                    )));
                }
            }
        }
    }

    // macro_rules! unexpected_first {
    //   ($block:ident) => {
    //     if state.i == 0 {
    //       return Err(Error::InvalidLlmsTxtFormat(format!("Only expecting to see a H1 as the first element, not a {}.", block)))
    //     }
    //   }
    // }

    let mut state = S::initial();
    for block in doc.blocks.iter() {
        match block {
            Paragraph(inline_segments) => {
                // unexpected_first!(block);
                if state.i == 0 {
                    return Err(Error::InvalidLlmsTxtFormat(
                        "Only expecting to see a H1 as the first element, not a Paragraph.".into(),
                    ));
                }

                use ast::Inline::*;
                for s in inline_segments.iter() {
                    match s {
                        Text(text) => {
                            // Ok
                        }

                        // Hard line break
                        LineBreak => {
                            if state.stage != Stage::LookingForOptionalDetails {
                                return Err(Error::InvalidLlmsTxtFormat(
                                    "Found a line break outside of the optional details section."
                                        .into(),
                                ));
                            }
                        }

                        // Inline code span
                        Code(code) => {
                            // Ok
                        }

                        // Raw HTML fragment
                        Html(html) => {
                            if state.stage != Stage::LookingForOptionalDetails {
                                return Err(Error::InvalidLlmsTxtFormat(format!(
                                    "Found an HTML fragment outside of the optional details section: '{}'",
                                    html
                                )));
                            }
                        }

                        // Link to a destination with optional title.
                        Link(ast::Link {
                            destination: _,
                            title: _,
                            children: _,
                        }) => {
                            // Ok
                        }

                        // Reference link
                        LinkReference(ast::LinkReference { label: _, text: _ }) => {
                            // ok
                        }

                        // Image with optional title.
                        Image(ast::Image {
                            destination,
                            title,
                            alt,
                        }) => {
                            if state.stage != Stage::LookingForOptionalDetails {
                                return Err(Error::InvalidLlmsTxtFormat(format!(
                                    "Found image outside of optional details section | destination: '{}', title: '{}', alt: '{}'",
                                    destination,
                                    title.unwrap_or_default(),
                                    alt
                                )));
                            }
                        }

                        // Emphasis (`*` / `_`)
                        Emphasis(_inline_segments) => {
                            // Ok
                        }

                        // Strong emphasis (`**` / `__`)
                        Strong(_inline_segments) => {
                            // Ok
                        }

                        // Strikethrough (`~~`)
                        Strikethrough(_inline_segments) => {
                            // Ok
                        }

                        // Autolink (`<https://>` or `<mailto:…>`)
                        Autolink(_link) => {
                            // Ok
                        }

                        // Footnote reference (`[^label]`)
                        FootnoteReference(_footnote) => {
                            // Ok
                        }

                        // Empty element. This is used to represent skipped elements in the AST.
                        Empty => {
                            // Ok
                        }
                    }
                }
            }

            // ATX (`# Heading`) or Setext (`===`) heading
            Heading(ast::Heading { kind, content }) => {
                use ast::HeadingKind::*;
                use ast::SetextHeading::*;
                match kind {
                    Atx(h_num) => {
                        if h_num == 1 {
                            state.accept_h1(content)?;
                        } else if h_num == 2 {
                            state.accept_other_header()?;
                        } else {
                            return Err(Error::InvalidLlmsTxtFormat(format!(
                                "Can only accept H2 headers in the file lists section. Invalid H{}: '{:?}'",
                                h_num, content
                            )));
                        }
                    }
                    Setext(h_num) => match h_num {
                        Level1 => {
                            state.accept_h1(content)?;
                        }
                        Level2 => {
                            state.accept_other_header()?;
                        }
                    },
                }
            }

            // Thematic break (horizontal rule)
            ThematicBreak => {
                if state.stage != Stage::LookingForOptionalDetails {
                    return Err(Error::InvalidLlmsTxtFormat(
                        "Found a thematic break outside of the optional details section.".into(),
                    ));
                }
            }

            // Block quote
            BlockQuote(blocks) => {
                match state.stage {
                    Stage::LookingForSummaryBlockquote => {
                        // found the (required-ish) summary blockquote!
                        state.has_summary_blockquote = true;
                        state.stage = Stage::LookingForOptionalDetails;
                    }
                    Stage::LookingForOptionalDetails => {
                        // OK to have anything other than a heading in the optional details section
                    }
                    wrong_stage => {
                        return Err(Error::InvalidLlmsTxtFormat(format!(
                            "Found a BlockQuote outside in the wrong stage {}: '{:?}'",
                            wrong_stage, blocks
                        )));
                    }
                }
            }

            // List (bullet or ordered)
            List(ast::List { kind, items }) => {
                match stage {
                    Stage::LookingForOptionalDetails | Stage::LookingForFileListSections => {
                        // ok to have these here
                    }
                    wrong_stage => {
                        return Err(Error::InvalidLlmsTxtFormat(format!(
                            "Found a List in the wrong stage {} (only optional details or file list): {:?} of '{:?}'",
                            wrong_stage, kind, items
                        )));
                    }
                }
            }

            // Fenced or indented code block
            CodeBlock(ast::CodeBlock { kind, literal }) => {
                if state.stage != Stage::LookingForOptionalDetails {
                    return Err(Error::InvalidLlmsTxtFormat(format!(
                        "Found a code block outside of the optional details section: {:?} {:?}",
                        kind, literal
                    )));
                }
            }

            // Raw HTML block
            HtmlBlock(html) => {
                if state.stage != Stage::LookingForOptionalDetails {
                    return Err(Error::InvalidLlmsTxtFormat(format!(
                        "Found an HTML block outside of the optional details section: '{}'",
                        html
                    )));
                }
            }

            // Link reference definition.  Preserved for round‑tripping.
            Definition(ast::LinkDefinition {
                label,
                destination,
                title,
            }) => {
                if state.stage != Stage::LookingForOptionalDetails {
                    return Err(Error::InvalidLlmsTxtFormat(format!(
                        "Found a Link definition outside of the optional details section | label: '{}', destination: '{}', title: '{}'",
                        label, destination, title
                    )));
                }
            }

            // Tables
            Table(ast::Table { rows, alignments }) => {
                if state.stage != Stage::LookingForOptionalDetails {
                    return Err(Error::InvalidLlmsTxtFormat(format!(
                        "Found a table outside of the optional details section | rows: {:?}, alignments: {:?}",
                        rows, alignments
                    )));
                }
            }

            // Footnote definition
            FootnoteDefinition(ast::FootnoteDefinition { label, blocks }) => {
                if state.stage != Stage::LookingForOptionalDetails {
                    return Err(Error::InvalidLlmsTxtFormat(format!(
                        "Found a footnote definition outside of the optional details section | label: '{}', blocks: {:?}",
                        label, blocks
                    )));
                }
            }

            // GitHub alert block (NOTE, TIP, IMPORTANT, WARNING, CAUTION)
            GitHubAlert(ast::GitHubAlert { alert_type, blocks }) => {
                if state.stage != Stage::LookingForOptionalDetails {
                    return Err(Error::InvalidLlmsTxtFormat(format!(
                        "Found a GitHub style alert outside of the optional details section | type: {:?}, blocks: {:?}",
                        alert_type, blocks
                    )));
                }
                // use ast::GitHubAlertType::*;
                // match alert_type {
                //     Note => {
                //         unimplemented!()
                //     }
                //     Tip => {
                //         unimplemented!()
                //     }
                //     Important => {
                //         unimplemented!()
                //     }
                //     Warning => {
                //         unimplemented!()
                //     }
                //     Caution => {
                //         unimplemented!()
                //     },
                //     Custom(label) => {
                //       unimplemented!();
                //     }
                // }
            }

            // Empty block. This is used to represent skipped blocks in the AST.
            Empty => {
                // allow empty blocks anywhere
            }
        }
        state.i += 1;
    }

    Ok(LlmTxt(doc))
}
