/// True if the environment variable is set and not empty. False otherwise.
pub fn is_env_set(env_var: &str) -> bool {
    match std::env::var(env_var) {
        Ok(val) => !val.is_empty(),
        Err(_) => false,
    }
}
