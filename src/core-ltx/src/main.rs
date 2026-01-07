use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use core_ltx::{
    is_valid_markdown,
    llms::{ChatGpt, LlmProvider},
    validate_is_llm_txt,
};

#[derive(Parser)]
#[command(name = "core-llmstxt")]
#[command(about = "The Core llms.txt Toolkit", long_about = None)]
struct CoreCli {
    #[command(subcommand)]
    command: Commands,
    /// Output file path for the generated llms.txt
    #[arg(short, long, value_parser = validate_output_file)]
    output: PathBuf,
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
        provider: LlmProviders,
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
        provider: LlmProviders,
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

    if path.exists() && path.is_dir() {
        return Err(format!("Output path is a directory: {}", path.display()));
    }

    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        return Err(format!(
            "Output file parent directory does not exist: {}",
            parent.display()
        ));
    }

    Ok(path)
}

#[tokio::main]
async fn main() {
    let cli = CoreCli::parse();

    match &cli.command {
        Commands::Markdown { file } => match std::fs::read_to_string(file) {
            Ok(content) => match is_valid_markdown(&content) {
                Ok(_doc) => println!("Valid markdown file: {file:?}"),
                Err(e) => println!("Invalid markdown file ({file:?}):\n{e:?}"),
            },
            Err(e) => println!("Cannot read file ({file:?}) due to: {e:?}"),
        },

        Commands::Validate { file } => match std::fs::read_to_string(file) {
            Ok(content) => match is_valid_markdown(&content) {
                Ok(doc) => match validate_is_llm_txt(doc) {
                    Ok(_llms_txt) => println!("Valid llms.txt file: {file:?}"),
                    Err(e) => println!("Invalid llms.txt file ({file:?}): {e:?}"),
                },
                Err(e) => println!("Invalid llms.txt file because it's an invalid markdown file ({file:?}):\n{e:?}"),
            },
            Err(e) => {
                println!("Cannot read file ({file:?}) due to: {e:?}");
                std::process::exit(1)
            }
        },

        Commands::Generate { website, provider } => {
            let html = website_content(website).await;
            let llm_provider = provider.provider();
            let llms_txt = core_ltx::llms::generate_llms_txt(llm_provider.as_ref(), &html)
                .await
                .unwrap();
            println!("{}", llms_txt.to_string());
        }

        Commands::Update {
            website,
            llms_txt,
            provider,
        } => {
            let web_content = website_content(website).await;

            let llms_txt_content = match std::fs::read_to_string(llms_txt) {
                Ok(x) => x,
                Err(e) => {
                    println!("ERROR: Cannot read file ({llms_txt:?}) due to: {e:?}");
                    std::process::exit(1)
                }
            };

            unimplemented!(
                "update llms.txt [1] with website [2]:\n[1]\n{llms_txt_content}\n[2]\n{web_content}\n[3] {provider:?}"
            );
        }
    }
}

async fn website_content(website: &Website) -> String {
    if let Some(file) = &website.file {
        match std::fs::read_to_string(file) {
            Ok(content) => content,
            Err(e) => {
                println!("ERROR: Cannot read file ({file:?}) due to: {e:?}");
                std::process::exit(1)
            }
        }
    } else if let Some(url) = &website.url {
        let validated_url = core_ltx::is_valid_url(url.as_str()).unwrap();
        core_ltx::download(&validated_url).await.unwrap()
    } else {
        unreachable!("Clap should enforce that exactly one option is provided")
    }
}
