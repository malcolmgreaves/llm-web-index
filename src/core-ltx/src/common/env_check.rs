/// Ensures that the listed environment variables exist and are non-empty.
/// Panics on error.
pub fn check_non_empty_env_vars(required_vars: &[&str]) {
    let mut any_error: u32 = 0;
    for var_name in required_vars {
        let var_name = *var_name;
        match std::env::var(var_name) {
            Ok(value) if !value.trim().is_empty() => {
                // Variable is present and non-empty, continue
            }
            Ok(_) => {
                eprintln!(
                    "FATAL: {} environment variable is set but empty. \
                   Authentication is enabled (ENABLE_AUTH=true) but required configuration is invalid.",
                    var_name
                );
                any_error += 1;
            }
            Err(_) => {
                if var_name == "AUTH_PASSWORD_HASH" {
                    eprintln!("Generate a hash with: cargo run --bin generate-password-hash -- your_password");
                } else if var_name == "SESSION_SECRET" {
                    eprintln!("Generate a secret with: openssl rand -base64 32");
                }
                eprintln!(
                    "FATAL: {} environment variable is required when ENABLE_AUTH=true.",
                    var_name
                );
                any_error += 1;
            }
        }
    }
    if any_error > 0 {
        panic!("{} environment variables failed checks.", any_error);
    }
}
