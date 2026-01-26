use markdown_ppp::ast::{self};
use markdown_ppp::parser::{MarkdownParserState, parse_markdown};
use markdown_ppp::printer::{config::Config, render_markdown};

use crate::Error;

/// A markdown document, represented as an abstract syntax tree (AST) of markdown blocks.
pub type Markdown = ast::Document;

/// Parses the text as markdown, returning a Markdown AST. Otherwise, produces an error explaining why the text isn't valid markdown.
pub fn is_valid_markdown(content: &str) -> Result<Markdown, Error> {
    match parse_markdown(MarkdownParserState::default(), content) {
        Err(error) => Err(Error::InvalidMarkdown(error.to_owned())),
        Ok(document) => Ok(document),
    }
}

/// A valid llms.txt file, described by a markdown document.
#[derive(Debug, Clone)]
pub struct LlmsTxt(Markdown);

/// The only way to make an LlmTxt is to validate it with `validate_is_llm_txt`.
impl LlmsTxt {
    /// Provide access to the underlying llms.txt markdown document.
    pub fn map<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&Markdown) -> T,
    {
        f(&self.0)
    }

    /// Destroy the LlmTxt wrapper, extracting the underlying markdown AST.
    pub fn extract(self) -> Markdown {
        self.0
    }

    /// Gets the Markdown content as a string.
    pub fn md_content(&self) -> String {
        render_markdown(&self.0, Config::default())
    }
}

/// Determines whether or not the markdown document adheres to the llms.txt specification.
///
/// This function is the only way to make an `LlmTxt` instance.
pub fn validate_is_llm_txt(doc: Markdown) -> Result<LlmsTxt, Error> {
    use ast::Block::*;

    #[derive(PartialEq, Eq, Copy, Clone)]
    enum Stage {
        LookingForH1,
        LookingForSummaryBlockquote,
        LookingForOptionalDetails,
        // LookingForFileListSections,
        LookingForFileListSectionsNeedList,
        LookingForFileListSectionsNeedListOrH2,
    }

    impl std::fmt::Display for Stage {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                Stage::LookingForH1 => write!(f, "Looking for H1"),
                Stage::LookingForSummaryBlockquote => write!(f, "Looking for Summary Blockquote"),
                Stage::LookingForOptionalDetails => {
                    write!(f, "Looking for Optional Detail Section(s)")
                }
                Stage::LookingForFileListSectionsNeedList => {
                    write!(
                        f,
                        "Looking for Optional File List Section(s): Need to find a List element"
                    )
                }
                Stage::LookingForFileListSectionsNeedListOrH2 => {
                    write!(
                        f,
                        "Looking for Optional File List Section(s): Need to continue a list or start a new section"
                    )
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
    }

    type Step = Result<(), Error>;

    impl S {
        fn initial() -> Self {
            Self {
                i: 0,
                stage: Stage::LookingForH1,
                has_h1_name_site: false,
                has_summary_blockquote: false,
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
                Stage::LookingForFileListSectionsNeedListOrH2 | Stage::LookingForOptionalDetails => {
                    // accept: make sure we stay in the file list stage (we could skip over the optional details)
                    // we just saw the H2, so we need to see a list element
                    self.stage = Stage::LookingForFileListSectionsNeedList;
                    Ok(())
                }
                wrong_stage => Err(Error::InvalidLlmsTxtFormat(format!(
                    "Found a header when we were not looking for file lists! We are looking for: {}",
                    wrong_stage
                ))),
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
                        Text(_text) => {
                            // Ok
                        }

                        // Hard line break
                        LineBreak => {
                            if state.stage != Stage::LookingForOptionalDetails {
                                return Err(Error::InvalidLlmsTxtFormat(
                                    "Found a line break outside of the optional details section.".into(),
                                ));
                            }
                        }

                        // Inline code span
                        Code(_code) => {
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
                                    title.clone().unwrap_or("".to_string()),
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
                        if *h_num == 1 {
                            state.accept_h1(content)?;
                        } else if *h_num == 2 {
                            state.accept_other_header()?;
                        } else {
                            return Err(Error::InvalidLlmsTxtFormat(format!(
                                "Can only accept H2 headers in the file lists section. Invalid H{}: '{:?}'",
                                *h_num, content
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
                match state.stage {
                    Stage::LookingForOptionalDetails => {
                        // ok to have here
                    }
                    Stage::LookingForFileListSectionsNeedList | Stage::LookingForFileListSectionsNeedListOrH2 => {
                        state.stage = Stage::LookingForFileListSectionsNeedListOrH2;
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
                        "Found a Link definition outside of the optional details section | label: '{:?}', destination: '{}', title: '{:?}'",
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

    state.final_validation()?;

    Ok(LlmsTxt(doc))
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use markdown_ppp::ast::Block;

    use super::*;

    #[test]
    fn markdown_validation() {
        assert!(is_valid_markdown("").is_ok());

        let x = is_valid_markdown("# Title");
        assert!(x.is_ok());
        match x.unwrap().blocks.first().unwrap() {
            Block::Heading(ast::Heading {
                kind: ast::HeadingKind::Atx(1),
                content,
            }) => match content.first().unwrap() {
                ast::Inline::Text(text) => assert_eq!(text, "Title"),
                _ => panic!("unexpected text: {:?}", content),
            },
            x => panic!("unexpected block type: {:?}", x),
        }

        assert!(
            is_valid_markdown("# Title\n- a list\n-with more\n-than one element\n```hello world!\nhow are you?\n```\n")
                .is_ok()
        )
    }

    #[test]
    fn llm_txt_validation() {
        // minimally ok
        assert!(validate_is_llm_txt(is_valid_markdown("# a title\n>>>> blockquote section").unwrap()).is_ok());

        // maxmimal example
        assert!(
            validate_is_llm_txt(
                is_valid_markdown(indoc! { "
            # a title
            >>>> blockquote
            >>>> section
            >>>> here

            - something else here
            1. is
            2. ok
            We just **cannot** have a section heading here!

            ## One we are in the file lists
            - we
            - are
            - ok

            ## note that we
            - do not

            ## check
            - that each list element here is link format
            - which we really _should_ do
          "})
                .unwrap()
            )
            .is_ok()
        );

        // missing everything
        assert!(validate_is_llm_txt(is_valid_markdown("").unwrap()).is_err());

        // missing blockquote summary
        assert!(validate_is_llm_txt(is_valid_markdown("# a title").unwrap()).is_err());

        // has an invalid header section
        assert!(
            validate_is_llm_txt(
                is_valid_markdown(indoc! { "
            # a title
            >>>> blockquote
            >>>> section
            >>>> here

            - something else here
            1. is
            2. ok

            ### We just **cannot** have a section heading here!

            ## One we are in the file lists
            - we
            - are
            - ok

            ## note that we
            - do not

            ## check
            - that each list element here is link format
            - which we really _should_ do
          "})
                .unwrap()
            )
            .is_err()
        );
    }
}
