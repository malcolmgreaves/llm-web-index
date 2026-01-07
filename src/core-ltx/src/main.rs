use std::{fmt::Debug, path::PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};
use core_ltx::{is_valid_markdown, llms::LlmProvider, validate_is_llm_txt};

#[derive(Parser)]
#[command(name = "core-llmstxt")]
#[command(about = "The Core llms.txt Toolkit", long_about = None)]
struct CoreCli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "lowercase")]
enum LlmProviders {
    ChatGpt,
    Claude,
}

impl LlmProviders {
    pub fn provider(&self) -> Box<dyn LlmProvider> {
        Box::new(match self {
            LlmProviders::ChatGpt => core_ltx::llms::ChatGpt::default(),
            LlmProviders::Claude => unimplemented!("implement Claude LLM provider"),
        })
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and validate a file as markdown
    Markdown {
        /// The file to parse and validate.
        #[arg(short, long)]
        file: PathBuf,
    },

    /// Validate that a file is a valid llms.txt.
    Validate {
        #[arg(short, long)]
        file: PathBuf,
    },

    /// Generate a new llms.txt from a website
    Generate {
        /// The website to generate an llms.txt file for.
        #[command(flatten)]
        website: Website,

        /// The LLM provider to use for generation
        #[arg(short, long)]
        provider: LlmProviders,

        /// Output file path for the generated llms.txt
        #[arg(short, long, value_parser = validate_output_file)]
        output: PathBuf,
    },

    /// Update an existing llms.txt
    Update {
        /// The website to generate an updated llms.txt file for.
        #[command(flatten)]
        website: Website,

        /// The prior existing llms.txt file.
        #[arg(short, long, value_parser = validate_input_file)]
        llms_txt: PathBuf,

        /// The LLM provider to use for generation
        #[arg(short, long)]
        provider: LlmProviders,

        /// Output file path for the updated llms.txt
        #[arg(short, long, value_parser = validate_output_file)]
        output: PathBuf,
    },
}

#[derive(Clone, Args)]
#[group(required = true, multiple = false)]
struct Website {
    /// The website URL to download and generate an llms.txt for.
    #[arg(short, long, group = "website")]
    url: Option<String>,
    /// The local filepath of HTML of a pre-downloaded webpage to generate an llms.txt for.
    #[arg(short, long, group = "website")]
    file: Option<PathBuf>,
}

fn validate_input_file(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);

    if !path.exists() {
        return Err(format!("Input path does not exist: {}", path.display()));
    }

    if !path.is_file() {
        return Err(format!("Input path is not a file: {}", path.display()));
    }

    let metadata = std::fs::metadata(&path).map_err(|e| format!("Cannot read update file metadata: {}", e))?;

    if metadata.len() == 0 {
        return Err(format!("Input file is empty: {}", path.display()));
    }

    Ok(path)
}

fn validate_output_file(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);

    if path.is_dir() {
        return Err(format!("Output path is a directory: {}", path.display()));
    }

    // let path = path
    //     .canonicalize()
    //     .map_err(|e| format!("Cannot canonicalize output path: {}", e))?;

    // if let Some(parent) = path.parent()
    //     && !parent.exists()
    // {
    //     return Err(format!(
    //         "Output file parent directory does not exist: {}",
    //         parent.display()
    //     ));
    // }

    Ok(path)
}

struct MainError(String);

impl Debug for MainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for MainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MainError {}

impl From<core_ltx::Error> for MainError {
    fn from(e: core_ltx::Error) -> Self {
        Self(e.to_string())
    }
}

impl From<std::io::Error> for MainError {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), MainError> {
    let cli = CoreCli::parse();

    match &cli.command {
        Commands::Markdown { file } => match std::fs::read_to_string(file) {
            Ok(content) => match is_valid_markdown(&content) {
                Ok(_doc) => println!("Valid markdown file: {file:?}"),
                Err(e) => println!("[ERROR] Invalid markdown file ({file:?}):\n{e:?}"),
            },
            Err(e) => return Err(MainError(format!("Cannot read file ({file:?}) due to: {e:?}"))),
        },

        Commands::Validate { file } => match std::fs::read_to_string(file) {
            Ok(content) => match is_valid_markdown(&content) {
                Ok(doc) => match validate_is_llm_txt(doc) {
                    Ok(_llms_txt) => println!("Valid llms.txt file: {file:?}"),
                    Err(e) => println!("[ERROR] Invalid llms.txt file ({file:?}): {e:?}"),
                },
                Err(e) => {
                    println!("[ERROR] Invalid llms.txt file because it's an invalid markdown file ({file:?}):\n{e:?}")
                }
            },
            Err(e) => {
                return Err(MainError(format!("Cannot read file ({file:?}) due to: {e:?}")));
            }
        },

        Commands::Generate {
            website,
            provider,
            output,
        } => {
            let html = website_content(website).await?;
            let llm_provider = provider.provider();
            let llms_txt = core_ltx::llms::generate_llms_txt(&*llm_provider, &html).await?;
            let as_markdown = llms_txt.md_content();
            std::fs::write(output, &as_markdown)?;
        }

        Commands::Update {
            website,
            llms_txt,
            provider,
            output,
        } => {
            let html = website_content(website).await?;
            let llms_txt_content = std::fs::read_to_string(llms_txt)?;
            let llm_provider = provider.provider();
            let updated_llms_txt = core_ltx::llms::update_llms_txt(&*llm_provider, &llms_txt_content, &html).await?;
            let as_markdown = updated_llms_txt.md_content();
            std::fs::write(output, &as_markdown)?;
        }
    }
    Ok(())
}

async fn website_content(website: &Website) -> Result<String, MainError> {
    if let Some(file) = &website.file {
        let content = std::fs::read_to_string(file)?;
        Ok(content)
    } else if let Some(url) = &website.url {
        let validated_url = core_ltx::is_valid_url(url.as_str())?;
        let html = core_ltx::download(&validated_url).await?;
        Ok(html)
    } else {
        unreachable!("Clap should enforce that exactly one option is provided")
    }
}
