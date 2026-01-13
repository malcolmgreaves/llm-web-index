//! Main generator functions for creating llms.txt documentation.

use crate::rule_gen::batch::process_in_batches;
use crate::rule_gen::config::GeneratorOptions;
use crate::rule_gen::errors::Result;
use crate::rule_gen::fetch::{fetch_html, fetch_sitemap};
use crate::rule_gen::html::{extract_main_content, get_description, get_title};
use crate::rule_gen::text_utils::{capitalize_string, clean_title, substitute_title};
use crate::rule_gen::url_utils::{build_url_filters, parse_section, should_process_url};
use std::collections::HashMap;

/// Page information extracted during processing.
#[derive(Debug, Clone)]
struct PageInfo {
    title: String,
    url: String,
    description: Option<String>,
    section: String,
}

/// Page information with full content for gen_full.
#[derive(Debug, Clone)]
struct FullPageInfo {
    title: String,
    url: String,
    description: Option<String>,
    markdown: String,
    anchor: String,
    lastmod: Option<String>,
}

/// Generates llms.txt documentation from a sitemap URL.
///
/// This function:
/// 1. Fetches the sitemap
/// 2. Filters URLs based on include/exclude patterns
/// 3. Processes each URL to extract title and description
/// 4. Organizes content by URL sections
/// 5. Generates markdown documentation
///
/// # Arguments
///
/// * `sitemap_url` - URL of the sitemap to process
/// * `options` - Configuration options for generation
///
/// # Returns
///
/// A markdown-formatted string containing the generated documentation.
///
/// # Errors
///
/// Returns an error if:
/// - The sitemap cannot be fetched or parsed
/// - Network requests fail
/// - Invalid configuration options are provided
///
/// # Examples
///
/// ```no_run
/// # use rule_llms_txt_gen::{gen, GeneratorOptions};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let options = GeneratorOptions::builder()
///     .concurrency(10)
///     .exclude_path("*/admin/*".to_string())
///     .build();
///
/// let output = gen("https://example.com/sitemap.xml", options).await?;
/// println!("{}", output);
/// # Ok(())
/// # }
/// ```
pub async fn gen(sitemap_url: &str, options: GeneratorOptions) -> Result<String> {
    // Fetch sitemap
    let sitemap = fetch_sitemap(sitemap_url).await?;
    let urls = sitemap.sites();

    // Build URL filters
    let (exclude_glob, include_glob) =
        build_url_filters(&options.include_paths, &options.exclude_paths)?;

    // Process URLs in batches
    let pages = process_in_batches(
        urls,
        move |url, _index| {
            let exclude_glob = exclude_glob.clone();
            let include_glob = include_glob.clone();
            let replace_titles = options.replace_title.clone();

            Box::pin(async move {
                // Check if URL should be processed
                if !should_process_url(&url, &exclude_glob, &include_glob) {
                    return None;
                }

                // Fetch HTML
                let html = fetch_html(&url).await.ok()?;

                // Extract title
                let mut title = get_title(&html)?;

                // Apply title substitutions
                for command in &replace_titles {
                    title = substitute_title(&title, command).ok()?;
                }
                title = clean_title(&title);

                // Extract description
                let description = get_description(&html);

                // Parse section
                let section = parse_section(&url);

                Some(PageInfo {
                    title,
                    url,
                    description,
                    section,
                })
            })
        },
        options.concurrency,
    )
    .await;

    // Organize pages by section
    let mut sections: HashMap<String, Vec<PageInfo>> = HashMap::new();
    for page in pages {
        sections
            .entry(page.section.clone())
            .or_insert_with(Vec::new)
            .push(page);
    }

    // Generate output
    let mut output = String::new();

    // Handle root section
    let root = sections.remove("ROOT").unwrap_or_default();

    // Determine title and description
    let doc_title = options
        .title
        .or_else(|| root.first().map(|p| p.title.clone()))
        .unwrap_or_else(|| "Documentation".to_string());

    let doc_description = options
        .description
        .or_else(|| root.first().and_then(|p| p.description.clone()))
        .unwrap_or_else(|| "Generated documentation".to_string());

    // Write header
    output.push_str(&format!("# {}\n\n", doc_title));
    output.push_str(&format!("> {}\n\n", doc_description));

    // Write sections
    let mut section_names: Vec<String> = sections.keys().cloned().collect();
    section_names.sort();

    for section_name in section_names {
        if let Some(pages) = sections.get(&section_name) {
            output.push_str(&format!("## {}\n\n", capitalize_string(&section_name)));

            for page in pages {
                output.push_str(&format!("- [{}]({})", page.title, page.url));
                if let Some(desc) = &page.description {
                    output.push_str(&format!(": {}", desc));
                }
                output.push('\n');
            }

            output.push('\n');
        }
    }

    Ok(output)
}

/// Generates full llms.txt documentation with complete page content.
///
/// Similar to `gen()`, but also extracts and includes the full content
/// of each page as markdown.
///
/// This function:
/// 1. Fetches the sitemap
/// 2. Filters URLs based on include/exclude patterns
/// 3. Extracts title, description, and full content from each page
/// 4. Converts HTML content to markdown
/// 5. Builds a table of contents
/// 6. Generates comprehensive markdown documentation
///
/// # Arguments
///
/// * `sitemap_url` - URL of the sitemap to process
/// * `options` - Configuration options for generation
///
/// # Returns
///
/// A markdown-formatted string containing the full documentation with page content.
///
/// # Errors
///
/// Returns an error if:
/// - The sitemap cannot be fetched or parsed
/// - Network requests fail
/// - Invalid configuration options are provided
///
/// # Examples
///
/// ```no_run
/// # use rule_llms_txt_gen::{gen_full, GeneratorOptions};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let options = GeneratorOptions::builder()
///     .concurrency(5)
///     .title("Complete Documentation".to_string())
///     .build();
///
/// let output = gen_full("https://example.com/sitemap.xml", options).await?;
/// println!("{}", output);
/// # Ok(())
/// # }
/// ```
pub async fn gen_full(sitemap_url: &str, options: GeneratorOptions) -> Result<String> {
    // Fetch sitemap
    let sitemap = fetch_sitemap(sitemap_url).await?;
    let urls = sitemap.urls.clone();

    // Build lastmod map
    let mut lastmod_map: HashMap<String, String> = HashMap::new();
    for url_entry in &urls {
        if let Some(lastmod) = &url_entry.lastmod {
            lastmod_map.insert(url_entry.loc.clone(), lastmod.clone());
        }
    }

    let url_strings: Vec<String> = urls.iter().map(|u| u.loc.clone()).collect();

    // Build URL filters
    let (exclude_glob, include_glob) =
        build_url_filters(&options.include_paths, &options.exclude_paths)?;

    // Process URLs in batches
    let pages = process_in_batches(
        url_strings,
        move |url, _index| {
            let exclude_glob = exclude_glob.clone();
            let include_glob = include_glob.clone();
            let replace_titles = options.replace_title.clone();
            let lastmod_map = lastmod_map.clone();

            Box::pin(async move {
                // Check if URL should be processed
                if !should_process_url(&url, &exclude_glob, &include_glob) {
                    return None;
                }

                // Fetch HTML
                let html = fetch_html(&url).await.ok()?;

                // Extract title
                let mut title = get_title(&html)?;

                // Apply title substitutions
                for command in &replace_titles {
                    title = substitute_title(&title, command).ok()?;
                }
                title = clean_title(&title);

                // Extract description
                let description = get_description(&html);

                // Extract main content
                let main_html = extract_main_content(&html);

                // Convert to markdown
                let markdown = html2md::parse_html(&main_html);

                // Create anchor
                let anchor = title
                    .to_lowercase()
                    .chars()
                    .map(|c| if c.is_alphanumeric() { c } else { '-' })
                    .collect::<String>();

                // Get lastmod
                let lastmod = lastmod_map.get(&url).cloned();

                Some(FullPageInfo {
                    title,
                    url,
                    description,
                    markdown,
                    anchor,
                    lastmod,
                })
            })
        },
        options.concurrency,
    )
    .await;

    // Generate output
    let mut output = String::new();

    // Document title
    let doc_title = options
        .title
        .unwrap_or_else(|| "Full Documentation".to_string());
    output.push_str(&format!("# {}\n\n", doc_title));

    // Build table of contents
    output.push_str("# Table of Contents\n");
    for page in &pages {
        output.push_str(&format!("- [{}](#{})\n", page.title, page.anchor));
    }
    output.push('\n');

    // Write page sections
    for page in &pages {
        output.push_str("\n\n---\n\n");
        output.push_str(&format!("## {}\n\n", page.title));
        output.push_str(&format!("[{}]({})\n\n", page.url, page.url));

        if let Some(desc) = &page.description {
            output.push_str(&format!("> {}\n\n", desc));
        }

        if let Some(lastmod) = &page.lastmod {
            output.push_str(&format!("*Last modified: {}*\n\n", lastmod));
        }

        output.push_str(&page.markdown);
        output.push('\n');
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_info_creation() {
        let page = PageInfo {
            title: "Test Page".to_string(),
            url: "https://example.com/test".to_string(),
            description: Some("Test description".to_string()),
            section: "test".to_string(),
        };

        assert_eq!(page.title, "Test Page");
        assert_eq!(page.section, "test");
    }

    #[test]
    fn test_full_page_info_creation() {
        let page = FullPageInfo {
            title: "Test Page".to_string(),
            url: "https://example.com/test".to_string(),
            description: Some("Test description".to_string()),
            markdown: "# Content".to_string(),
            anchor: "test-page".to_string(),
            lastmod: Some("2024-01-01".to_string()),
        };

        assert_eq!(page.anchor, "test-page");
        assert_eq!(page.lastmod, Some("2024-01-01".to_string()));
    }
}
