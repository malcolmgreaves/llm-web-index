use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use core_ltx::{LlmsTxt, is_valid_markdown, validate_is_llm_txt};

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
    Generate(Website),

    /// Update an existing llms.txt
    Update {
        // The website to generate an updated llms.txt file for.
        website: Website,
        /// The prior existing llms.txt file.
        #[arg(short, long, value_parser = validate_input_file)]
        llms_txt: PathBuf,
    },
}

#[derive(Args)]
#[group(required = true, multiple = false)]
enum Website {
    /// The website URL to download and generate an llms.txt for.
    #[arg(short, long, group = "website")]
    Url(String),
    /// The local filepath of HTML of a pre-downloaded webpage to generate an llms.txt for.
    #[arg(short, long, group = "website")]
    File(PathBuf),
}

fn validate_url(s: &str) -> Result<String, String> {
    url::Url::parse(s)
        .map(|_| s.to_string())
        .map_err(|e| format!("Invalid URL: {}", e))
}

fn validate_input_file(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);

    if !path.exists() {
        return Err(format!("Input path does not exist: {}", path.display()));
    }

    if !path.is_file() {
        return Err(format!("Input path is not a file: {}", path.display()));
    }

    let metadata =
        std::fs::metadata(&path).map_err(|e| format!("Cannot read update file metadata: {}", e))?;

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

fn main() {
    let cli = CoreCli::parse();

    match &cli.command {
        Commands::Markdown { file } => match std::fs::read_to_string(&file) {
            Ok(content) => match is_valid_markdown(&content) {
                Ok(_doc) => println!("Valid markdown file: {file:?}"),
                Err(e) => println!("Invalid markdown file ({file:?}):\n{e:?}"),
            },
            Err(e) => println!("Cannot read file ({file:?}) due to: {e:?}"),
        },

        Commands::Validate { file } => match std::fs::read_to_string(&file) {
            Ok(content) => match is_valid_markdown(&content) {
                Ok(doc) => match validate_is_llm_txt(doc) {
                    Ok(_llms_txt) => println!("Valid llms.txt file: {file:?}"),
                    Err(e) => println!("Invalid llms.txt file ({file:?}): {e:?}"),
                },
                Err(e) => println!(
                    "Invalid llms.txt file because it's an invalid markdown file ({file:?}):\n{e:?}"
                ),
            },
            Err(e) => {
                println!("Cannot read file ({file:?}) due to: {e:?}");
                std::process::exit(1)
            }
        },

        Commands::Generate(website) => {
            let web_content = website_content(&website);
            unimplemented!("generate llms.txt from website content:\n{web_content}")
        }

        Commands::Update { website, llms_txt } => {
            let web_content = website_content(&website);

            let llms_txt_content = match std::fs::read_to_string(&llms_txt) {
                Ok(x) => x,
                Err(e) => {
                    println!("ERROR: Cannot read file ({llms_txt:?}) due to: {e:?}");
                    std::process::exit(1)
                }
            };

            unimplemented!(
                "update llms.txt [1] with website [2]:\n[1]\n{llms_txt_content}\n[2]\n{web_content}"
            );
        }
    }
}

fn website_content(website: &Website) -> String {
    match website {
        Website::File(file) => match std::fs::read_to_string(&file) {
            Ok(content) => content,
            Err(e) => {
                println!("ERROR: Cannot read file ({file:?}) due to: {e:?}");
                std::process::exit(1)
            }
        },
        Website::Url(url) => {
            unimplemented!("download URL ({url}) as file")
        }
    }
}
