//! URL parsing and filtering utilities.

use crate::rule_gen::errors::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use url::Url;

/// Parses the section name from a URL's pathname.
///
/// Returns the first path segment, or "ROOT" if the URL is at the root.
///
/// # Examples
///
/// ```
/// # use rule_llms_txt_gen::parse_section;
/// assert_eq!(parse_section("https://example.com/docs/guide.html"), "docs");
/// assert_eq!(parse_section("https://example.com/"), "ROOT");
/// assert_eq!(parse_section("invalid-url"), "ROOT");
/// ```
pub fn parse_section(uri: &str) -> String {
    match Url::parse(uri) {
        Ok(url) => {
            let segments: Vec<&str> = url.path_segments().map(|s| s.collect()).unwrap_or_default();

            segments
                .first()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "ROOT".to_string())
        }
        Err(_) => "ROOT".to_string(),
    }
}

/// Checks if a URL is at the root path (/).
///
/// # Examples
///
/// ```
/// # use rule_llms_txt_gen::is_root_url;
/// assert_eq!(is_root_url("https://example.com/"), true);
/// assert_eq!(is_root_url("https://example.com/docs"), false);
/// assert_eq!(is_root_url("invalid-url"), false);
/// ```
pub fn is_root_url(uri: &str) -> bool {
    match Url::parse(uri) {
        Ok(url) => url.path() == "/",
        Err(_) => false,
    }
}

/// Builds glob matchers for URL filtering.
///
/// Returns a tuple of (exclude_glob, include_glob).
/// - exclude_glob: Always present, matches URLs to exclude
/// - include_glob: Optional, if present only URLs matching this are included
///
/// # Errors
///
/// Returns an error if any glob pattern is invalid.
pub fn build_url_filters(include_paths: &[String], exclude_paths: &[String]) -> Result<(GlobSet, Option<GlobSet>)> {
    // Build exclude glob set
    let mut exclude_builder = GlobSetBuilder::new();
    for pattern in exclude_paths {
        exclude_builder.add(Glob::new(pattern)?);
    }
    let exclude_glob = exclude_builder.build()?;

    // Build include glob set if patterns are provided
    let include_glob = if !include_paths.is_empty() {
        let mut include_builder = GlobSetBuilder::new();
        for pattern in include_paths {
            include_builder.add(Glob::new(pattern)?);
        }
        Some(include_builder.build()?)
    } else {
        None
    };

    Ok((exclude_glob, include_glob))
}

/// Determines if a URL should be processed based on include/exclude filters.
///
/// # Arguments
///
/// * `url` - The URL to check
/// * `exclude_glob` - GlobSet for excluded patterns
/// * `include_glob` - Optional GlobSet for included patterns
///
/// # Returns
///
/// `true` if the URL should be processed, `false` otherwise.
///
/// # Logic
///
/// 1. If URL matches exclude pattern: exclude
/// 2. If include patterns exist and URL doesn't match any: exclude
/// 3. Otherwise: include
pub fn should_process_url(url: &str, exclude_glob: &GlobSet, include_glob: &Option<GlobSet>) -> bool {
    // Check if excluded
    if exclude_glob.is_match(url) {
        return false;
    }

    // Check if included (if include patterns are specified)
    if let Some(include) = include_glob {
        if !include.is_match(url) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_section() {
        assert_eq!(parse_section("https://example.com/docs/guide.html"), "docs");
        assert_eq!(parse_section("https://example.com/api/v1/users"), "api");
        assert_eq!(parse_section("https://example.com/"), "ROOT");
        assert_eq!(parse_section("https://example.com"), "ROOT");
        assert_eq!(parse_section("invalid-url"), "ROOT");
    }

    #[test]
    fn test_is_root_url() {
        assert!(is_root_url("https://example.com/"));
        assert!(!is_root_url("https://example.com/docs"));
        assert!(!is_root_url("https://example.com/docs/"));
        assert!(!is_root_url("invalid-url"));
    }

    #[test]
    fn test_build_url_filters() {
        let exclude = vec!["*/admin/*".to_string()];
        let include = vec!["*/docs/*".to_string()];

        let (exclude_glob, include_glob) = build_url_filters(&include, &exclude).unwrap();

        assert!(exclude_glob.is_match("https://example.com/admin/panel"));
        assert!(!exclude_glob.is_match("https://example.com/docs/guide"));

        assert!(include_glob.is_some());
        let include_glob = include_glob.unwrap();
        assert!(include_glob.is_match("https://example.com/docs/guide"));
        assert!(!include_glob.is_match("https://example.com/api/v1"));
    }

    #[test]
    fn test_should_process_url() {
        let exclude = vec!["*/admin/*".to_string()];
        let include = vec!["*/docs/*".to_string()];
        let (exclude_glob, include_glob) = build_url_filters(&include, &exclude).unwrap();

        // Should exclude admin URLs
        assert!(!should_process_url(
            "https://example.com/admin/panel",
            &exclude_glob,
            &include_glob
        ));

        // Should include docs URLs
        assert!(should_process_url(
            "https://example.com/docs/guide",
            &exclude_glob,
            &include_glob
        ));

        // Should exclude non-docs URLs when include filter is present
        assert!(!should_process_url(
            "https://example.com/api/v1",
            &exclude_glob,
            &include_glob
        ));
    }

    #[test]
    fn test_should_process_url_no_includes() {
        let exclude = vec!["*/admin/*".to_string()];
        let include = vec![];
        let (exclude_glob, include_glob) = build_url_filters(&include, &exclude).unwrap();

        // Should exclude admin URLs
        assert!(!should_process_url(
            "https://example.com/admin/panel",
            &exclude_glob,
            &include_glob
        ));

        // Should include everything else
        assert!(should_process_url(
            "https://example.com/api/v1",
            &exclude_glob,
            &include_glob
        ));
    }
}
