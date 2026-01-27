pub mod common;
pub mod errors;
pub mod functional;
pub mod llms;
pub mod md_llm_txt;
pub mod web_html;

pub use md_llm_txt::{LlmsTxt, Markdown, is_valid_markdown, validate_is_llm_txt};
pub use web_html::{clean_html, compute_html_checksum, download, is_valid_url, normalize_html, parse_html};

pub use common::auth_config::{AuthConfig, get_auth_config, is_auth_enabled};
pub use common::compression::{compress_string, decompress_to_string};
pub use common::db;
pub use common::db_env::get_db_pool;
pub use common::health::{health_check, health_router};
pub use common::hostname::{HostPortError, get_api_base_url};
pub use common::logging::setup_logging;
pub use common::max_concurrency::get_max_concurrency;
pub use common::poll_interval::{TimeUnit, get_poll_interval};
pub use common::tls_config::get_tls_config;

pub use errors::Error;
