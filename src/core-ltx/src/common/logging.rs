use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Sets the logging (tracing) level using RUST_LOG, falling back to the supplied default log settings.
pub fn setup_logging(default_log_settings: &str) {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| default_log_settings.into()))
        .with(tracing_subscriber::fmt::layer())
        .init()
}
