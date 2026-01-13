//! Text manipulation utilities.

use crate::rule_gen::errors::{LlmsGenError, Result};
use regex::Regex;

/// Capitalizes the first character of a string and lowercases the rest.
///
/// # Examples
///
/// ```
/// # use rule_llms_txt_gen::text_utils::capitalize_string;
/// assert_eq!(capitalize_string("hello"), "Hello");
/// assert_eq!(capitalize_string("WORLD"), "World");
/// assert_eq!(capitalize_string(""), "");
/// ```
pub fn capitalize_string(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }

    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first
            .to_uppercase()
            .chain(chars.as_str().to_lowercase().chars())
            .collect(),
    }
}

/// Removes leading pipe character and whitespace from a title.
///
/// # Examples
///
/// ```
/// # use rule_llms_txt_gen::text_utils::clean_title;
/// assert_eq!(clean_title("| Example Title"), "Example Title");
/// assert_eq!(clean_title("|   Example Title"), "Example Title");
/// assert_eq!(clean_title("Example Title"), "Example Title");
/// ```
pub fn clean_title(title: &str) -> String {
    title.trim_start_matches('|').trim().to_string()
}

/// Parses a sed-style substitution command into a regex and replacement string.
///
/// Format: `s/pattern/replacement/flags`
///
/// Supported flags:
/// - `i`: case insensitive
/// - `m`: multi-line mode
/// - `s`: dot matches newline
/// - `u`: Unicode support
/// - `g`: global replacement (handled via replace_all in Rust)
///
/// Note: The 'g' flag is ignored in the regex pattern since Rust's `replace_all()`
/// always replaces all matches. The 'y' flag is not supported.
///
/// # Examples
///
/// ```
/// # use rule_llms_txt_gen::text_utils::parse_substitution_command;
/// let (regex, replacement) = parse_substitution_command("s/foo/bar/g").unwrap();
/// assert_eq!(regex.replace_all("foo foo", &replacement), "bar bar");
/// ```
pub fn parse_substitution_command(command: &str) -> Result<(Regex, String)> {
    // Match s/pattern/replacement/flags format
    let re =
        Regex::new(r"^s/(.*?)/(.*?)/([gimsuy]*)$").map_err(|e| LlmsGenError::InvalidSubstitution(e.to_string()))?;

    let captures = re
        .captures(command)
        .ok_or_else(|| LlmsGenError::InvalidSubstitution("Invalid substitution command format".to_string()))?;

    let pattern = captures.get(1).map(|m| m.as_str()).unwrap_or("");
    let replacement = captures.get(2).map(|m| m.as_str()).unwrap_or("");
    let flags_str = captures.get(3).map(|m| m.as_str()).unwrap_or("");

    // Build regex with flags (filter out 'g' and 'y' which aren't valid inline flags)
    let rust_flags: String = flags_str
        .chars()
        .filter(|&c| c != 'g' && c != 'y') // 'g' is implicit with replace_all, 'y' is not supported
        .collect();

    let regex_pattern = if rust_flags.is_empty() {
        pattern.to_string()
    } else {
        format!("(?{}){}", rust_flags, pattern)
    };

    let regex = Regex::new(&regex_pattern)
        .map_err(|e| LlmsGenError::InvalidSubstitution(format!("Invalid regex pattern: {}", e)))?;

    Ok((regex, replacement.to_string()))
}

/// Applies a substitution command to a title.
///
/// # Examples
///
/// ```
/// # use rule_llms_txt_gen::text_utils::substitute_title;
/// assert_eq!(substitute_title("Hello World", "s/World/Rust/").unwrap(), "Hello Rust");
/// assert_eq!(substitute_title("foo foo", "s/foo/bar/g").unwrap(), "bar bar");
/// ```
pub fn substitute_title(title: &str, command: &str) -> Result<String> {
    if command.is_empty() || !command.starts_with("s/") {
        return Ok(title.to_string());
    }

    let (regex, replacement) = parse_substitution_command(command)?;
    Ok(regex.replace_all(title, replacement.as_str()).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalize_string() {
        assert_eq!(capitalize_string("hello"), "Hello");
        assert_eq!(capitalize_string("WORLD"), "World");
        assert_eq!(capitalize_string("hELLO"), "Hello");
        assert_eq!(capitalize_string(""), "");
        assert_eq!(capitalize_string("a"), "A");
    }

    #[test]
    fn test_clean_title() {
        assert_eq!(clean_title("| Example Title"), "Example Title");
        assert_eq!(clean_title("|   Example Title"), "Example Title");
        assert_eq!(clean_title("Example Title"), "Example Title");
        assert_eq!(clean_title(""), "");
        assert_eq!(clean_title("|"), "");
    }

    #[test]
    fn test_parse_substitution_command() {
        let (regex, replacement) = parse_substitution_command("s/foo/bar/").unwrap();
        assert_eq!(regex.replace("foo", &replacement), "bar");

        let (regex, replacement) = parse_substitution_command("s/foo/bar/g").unwrap();
        assert_eq!(regex.replace_all("foo foo", &replacement), "bar bar");
    }

    #[test]
    fn test_substitute_title() {
        assert_eq!(substitute_title("Hello World", "s/World/Rust/").unwrap(), "Hello Rust");
        assert_eq!(substitute_title("foo foo", "s/foo/bar/g").unwrap(), "bar bar");
        assert_eq!(substitute_title("Title", "").unwrap(), "Title");
        assert_eq!(substitute_title("Title", "not a command").unwrap(), "Title");
    }

    #[test]
    fn test_invalid_substitution() {
        assert!(parse_substitution_command("invalid").is_err());
        assert!(parse_substitution_command("s/foo").is_err());
    }
}
