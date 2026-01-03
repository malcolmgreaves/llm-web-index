use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "core-ltx")]
#[command(about = "CLI for generating and updating llms.txt files", long_about = None)]
struct Args {
    /// URL to process
    #[arg(short, long, value_parser = validate_url)]
    url: String,

    /// Path to prior llms.txt file to update
    #[arg(long, value_parser = validate_update_file)]
    update: PathBuf,

    /// Output file path for the generated llms.txt
    #[arg(short, long, value_parser = validate_output_file)]
    output: PathBuf,
}

fn validate_url(s: &str) -> Result<String, String> {
    url::Url::parse(s)
        .map(|_| s.to_string())
        .map_err(|e| format!("Invalid URL: {}", e))
}

fn validate_update_file(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);

    if !path.exists() {
        return Err(format!("Update file does not exist: {}", path.display()));
    }

    if !path.is_file() {
        return Err(format!("Update path is not a file: {}", path.display()));
    }

    let metadata =
        std::fs::metadata(&path).map_err(|e| format!("Cannot read update file metadata: {}", e))?;

    if metadata.len() == 0 {
        return Err(format!("Update file is empty: {}", path.display()));
    }

    Ok(path)
}

fn validate_output_file(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);

    if path.exists() && path.is_dir() {
        return Err(format!("Output path is a directory: {}", path.display()));
    }

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            return Err(format!(
                "Output file parent directory does not exist: {}",
                parent.display()
            ));
        }
    }

    Ok(path)
}

fn main() {
    let args = Args::parse();

    println!("URL: {}", args.url);
    println!("Update file: {}", args.update.display());
    println!("Output file: {}", args.output.display());

    panic!("unimplemented");
}
