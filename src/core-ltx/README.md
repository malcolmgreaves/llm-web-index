# core-ltx

The functional core of the llms.txt generation system. Contains all business logic for generating, validating, and updating llms.txt files from websites using LLM models. Also includes a CLI tool for standalone generation.

## Overview

The `core-ltx` crate is the heart of the application, providing:

- **Web content extraction**: Downloads and parses HTML from target websites
- **LLM integration**: Interfaces with OpenAI GPT models and Anthropic Claude models
- **llms.txt generation**: Transforms web content into structured llms.txt format
- **Validation and retry logic**: Ensures generated files meet format requirements
- **Update detection**: Compares existing llms.txt files with new versions
- **CLI tool**: Standalone command-line interface for one-off generation
- **Common utilities**: Shared configuration, logging, and helpers used by other crates

## Architecture

```
src/core-ltx/
├── src/
│   ├── lib.rs               # Core library exports
│   ├── main.rs              # CLI entry point
│   ├── errors.rs            # Error types
│   ├── llms/                # LLM model integrations
│   │   ├── mod.rs           # Model interface and generation logic
│   │   ├── chatgpt.rs       # OpenAI GPT integration
│   │   ├── claude.rs        # Anthropic Claude integration (placeholder)
│   │   └── prompts.rs       # System prompts for llms.txt generation
│   ├── web_html.rs          # HTML fetching and parsing
│   ├── md_llm_txt.rs        # Markdown/llms.txt format handling
│   └── common/              # Shared utilities
│       ├── mod.rs           # Common module exports
│       ├── auth_config.rs   # Authentication configuration helpers
│       ├── tls_config.rs    # TLS configuration helpers
│       ├── db_env.rs        # Database configuration helpers
│       ├── hostname.rs      # Hostname parsing utilities
│       ├── logging.rs       # Logging setup
│       └── poll_interval.rs # Polling interval configuration
├── Cargo.toml
└── build.rs                 # Build script for embedding prompts
```

## Key Features

### llms.txt Generation Pipeline

1. **Web Content Fetching**: Downloads HTML from the target URL
2. **HTML Parsing**: Extracts meaningful content using html5ever
3. **Content Preprocessing**: Cleans and structures the extracted text
4. **LLM Prompting**: Sends content to GPT-5.2 with specialized prompts
5. **Format Validation**: Ensures output conforms to llms.txt specification
6. **Retry Logic**: Automatically retries with fix prompts if validation fails
7. **Result Storage**: Returns generated content for storage/serving

### LLM Integration

Currently supports:
- **OpenAI GPT-5.2**: Primary model for generation (requires `OPENAI_API_KEY`)
- **OpenAI GPT-5 Mini**: Faster, more cost-effective option
- **OpenAI GPT-5 Nano**: Lightweight option for simple sites
- **Anthropic Claude**: Integration structure in place (not yet fully implemented)

The system uses carefully crafted prompts (see `src/llms/prompts.rs`) to ensure the generated llms.txt files:
- Follow the proper markdown format
- Include accurate summaries of the website
- Provide useful context for LLM consumption
- Maintain consistent structure

### Update Detection

When regenerating an llms.txt file:
1. Fetches current content from the website
2. Generates new llms.txt content
3. Compares with existing version
4. Determines if meaningful changes occurred
5. Only updates if content has substantively changed

This prevents unnecessary updates for minor formatting differences or timestamp changes.

### Common Utilities

The `common` module provides shared functionality used across all crates:

- **Auth configuration**: Parsing and validation of authentication settings
- **TLS configuration**: Loading and configuring TLS certificates
- **Database configuration**: PostgreSQL connection string parsing
- **Hostname utilities**: URL parsing and validation
- **Logging setup**: Structured logging with tracing
- **Poll intervals**: Configuration of periodic task intervals

## Configuration

The core library uses environment variables for configuration:

- `OPENAI_API_KEY`: OpenAI API key (required for generation)
- `RUST_LOG`: Logging level (default: `info`)

## Building

```bash
# Build the library and CLI
cargo build -p core-ltx

# Production build
cargo build -p core-ltx --release
```

## Running the CLI

The CLI tool allows standalone generation of llms.txt files:

```bash
# Basic usage: generate llms.txt for a website
cargo run -p core-ltx -- generate https://example.com

# Specify output file
cargo run -p core-ltx -- generate https://example.com --output example-llms.txt

# Update an existing llms.txt file
cargo run -p core-ltx -- update https://example.com --existing old-llms.txt

# Use different GPT model
cargo run -p core-ltx -- generate https://example.com --model gpt-5-mini

# View help
cargo run -p core-ltx -- --help
```

## Usage as a Library

```rust
use core_ltx::llms::{generate_llms_txt, LlmModel};
use core_ltx::web_html::fetch_and_parse_html;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Fetch website content
    let html_content = fetch_and_parse_html("https://example.com").await?;

    // Generate llms.txt using GPT-5.2
    let llms_txt = generate_llms_txt(
        "https://example.com",
        &html_content,
        LlmModel::Gpt52
    ).await?;

    println!("Generated llms.txt:\n{}", llms_txt);

    Ok(())
}
```

## Testing

```bash
# Run unit tests
cargo test -p core-ltx

# Run integration tests (requires OPENAI_API_KEY)
OPENAI_API_KEY=your_key cargo test -p core-ltx -- --ignored

# Run with coverage
just test
```

## Generation Prompts

The system uses multi-stage prompting:

1. **Initial generation prompt**: Describes the llms.txt format and requirements
2. **Fix prompts**: If validation fails, provides specific feedback for correction
3. **Update prompts**: For existing files, guides the model to detect meaningful changes

Prompts are embedded at compile time from template files and can be customized by modifying `src/llms/prompts.rs`.

## Error Handling

The crate defines comprehensive error types in `errors.rs`:

- `WebFetchError`: Problems downloading or parsing HTML
- `LlmError`: Issues communicating with LLM APIs
- `ValidationError`: llms.txt format validation failures
- `ConfigError`: Configuration or environment variable issues

All functions return `Result` types with descriptive errors.

## Dependencies

Key dependencies:

- `async-openai`: OpenAI API client
- `reqwest`: HTTP client for web fetching
- `html5ever`: HTML parsing
- `markup5ever_rcdom`: DOM representation for HTML
- `url`: URL parsing and validation
- `markdown-ppp`: Markdown preprocessing
- `nom`: Parser combinators for format validation
- `tokio`: Async runtime
- `tracing`: Structured logging

See [Cargo.toml](Cargo.toml) for the complete dependency list.

## Development

### Adding a New LLM Provider

To add support for a new LLM provider:

1. Create a new module in `src/llms/` (e.g., `gemini.rs`)
2. Implement the generation function following the existing pattern
3. Add the provider to the `LlmModel` enum in `src/llms/mod.rs`
4. Update the CLI argument parsing in `src/main.rs`
5. Add tests for the new provider

### Customizing Prompts

Prompts are defined in `src/llms/prompts.rs`. To customize:

1. Modify the prompt templates in `prompts.rs`
2. Test thoroughly to ensure generated content still validates
3. Consider adding prompt versioning for reproducibility

### Debugging Generation Issues

Enable detailed logging:

```bash
RUST_LOG=core_ltx=debug cargo run -p core-ltx -- generate https://example.com
```

This will show:
- HTTP request/response details
- Raw HTML content (truncated)
- LLM prompts sent
- LLM responses received
- Validation errors (if any)

## Performance Considerations

- Web fetching typically takes 1-3 seconds
- LLM generation typically takes 10-30 seconds
- Larger websites may require more processing time
- Consider using GPT-5-mini or GPT-5-nano for faster generation
- Connection pooling and request timeouts are configured for reliability

## Related Documentation

- [llmstxt.org](https://llmstxt.org) - Official llms.txt specification
- [Project Root README](../../README.md) - Overall project documentation
- [api-ltx README](../api-ltx/README.md) - API server documentation
- [worker-ltx README](../worker-ltx/README.md) - Worker service documentation
- [cron-ltx README](../cron-ltx/README.md) - Update scheduler documentation
