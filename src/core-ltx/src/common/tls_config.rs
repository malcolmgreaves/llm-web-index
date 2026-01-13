use axum_server::tls_rustls::RustlsConfig;
use std::env;
use std::path::PathBuf;

/// Get TLS configuration from environment variables
/// Panics if required configuration is missing or invalid
pub async fn get_tls_config() -> RustlsConfig {
    let cert_path_str = env::var("TLS_CERT_PATH").expect(
        "TLS_CERT_PATH environment variable is required. \
         Generate certificates with: ./make_tls_cert.sh",
    );

    let key_path_str = env::var("TLS_KEY_PATH").expect(
        "TLS_KEY_PATH environment variable is required. \
         Generate certificates with: ./make_tls_cert.sh",
    );

    let cert_path = PathBuf::from(&cert_path_str);
    let key_path = PathBuf::from(&key_path_str);

    // Validate files exist before loading
    if !cert_path.exists() {
        panic!(
            "Certificate file does not exist: {}\n\
             Generate certificates with: ./make_tls_cert.sh",
            cert_path.display()
        );
    }
    if !key_path.exists() {
        panic!(
            "Private key file does not exist: {}\n\
             Generate certificates with: ./make_tls_cert.sh",
            key_path.display()
        );
    }

    RustlsConfig::from_pem_file(cert_path, key_path)
        .await
        .expect("Failed to load TLS certificate and key")
}
