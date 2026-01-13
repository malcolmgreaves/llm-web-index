//! # llms.txt Generation Library
//!
//! A Rust library for generating llms.txt documentation from website sitemaps.
//!
//! This library fetches sitemaps, processes URLs with filtering rules, extracts metadata
//! and content, and generates markdown documentation suitable for LLM consumption.
//!
//! ## Features
//!
//! - Fetch and parse XML sitemaps
//! - Filter URLs with glob patterns (include/exclude)
//! - Extract page titles and descriptions
//! - Apply regex-based title transformations
//! - Generate summary documentation with links
//! - Generate full documentation with page content
//! - Concurrent processing with configurable limits
//!
//! ## Examples
//!
//! ### Basic Usage
//!
//! ```no_run
//! use rule_llms_txt_gen::{gen, GeneratorOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let options = GeneratorOptions::builder()
//!         .concurrency(10)
//!         .exclude_path("*/admin/*".to_string())
//!         .include_path("*/docs/*".to_string())
//!         .build();
//!
//!     let output = gen("https://example.com/sitemap.xml", options).await?;
//!     println!("{}", output);
//!     Ok(())
//! }
//! ```
//!
//! ### Generating Full Content
//!
//! ```no_run
//! use rule_llms_txt_gen::{gen_full, GeneratorOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let options = GeneratorOptions::builder()
//!         .concurrency(5)
//!         .title("Complete Documentation".to_string())
//!         .build();
//!
//!     let output = gen_full("https://example.com/sitemap.xml", options).await?;
//!     println!("{}", output);
//!     Ok(())
//! }
//! ```

// Module declarations
pub mod batch;
mod config;
mod errors;
mod fetch;
mod generator;
mod html;
pub mod text_utils;
mod url_utils;

// Public API re-exports
pub use config::{GeneratorOptions, GeneratorOptionsBuilder};
pub use errors::{LlmsGenError, Result};
pub use generator::{gen, gen_full};

// Additional exports for advanced usage
pub use fetch::{fetch_html, fetch_sitemap, Sitemap, SitemapUrl};
pub use html::{extract_main_content, get_description, get_title};
pub use url_utils::{build_url_filters, is_root_url, parse_section, should_process_url};
