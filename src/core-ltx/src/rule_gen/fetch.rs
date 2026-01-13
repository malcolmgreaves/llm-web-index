//! HTTP fetching and sitemap parsing utilities.

use crate::rule_gen::errors::{LlmsGenError, Result};
use quick_xml::events::Event;
use quick_xml::Reader;

/// Represents a sitemap with URLs and optional last modification dates.
#[derive(Debug, Clone)]
pub struct Sitemap {
    /// List of URLs from the sitemap
    pub urls: Vec<SitemapUrl>,
}

/// Represents a single URL entry in a sitemap.
#[derive(Debug, Clone)]
pub struct SitemapUrl {
    /// The URL location
    pub loc: String,
    /// Optional last modification date
    pub lastmod: Option<String>,
}

impl Sitemap {
    /// Returns a simple list of URL strings (for compatibility with JavaScript version)
    pub fn sites(&self) -> Vec<String> {
        self.urls.iter().map(|u| u.loc.clone()).collect()
    }
}

/// Fetches HTML content from a URL.
///
/// # Errors
///
/// Returns an error if the HTTP request fails or the response cannot be read.
///
/// # Examples
///
/// ```no_run
/// # use rule_llms_txt_gen::fetch_html;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let html = fetch_html("https://example.com").await?;
/// println!("Fetched {} bytes", html.len());
/// # Ok(())
/// # }
/// ```
pub async fn fetch_html(url: &str) -> Result<String> {
    let response = reqwest::get(url).await?;
    let text = response.text().await?;
    Ok(text)
}

/// Fetches and parses a sitemap from a URL.
///
/// Supports XML sitemaps in the standard format:
/// ```xml
/// <urlset>
///   <url>
///     <loc>https://example.com/page</loc>
///     <lastmod>2024-01-01</lastmod>
///   </url>
/// </urlset>
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP request fails
/// - The sitemap XML is malformed
/// - No URLs are found in the sitemap
///
/// # Examples
///
/// ```no_run
/// # use rule_llms_txt_gen::fetch_sitemap;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let sitemap = fetch_sitemap("https://example.com/sitemap.xml").await?;
/// println!("Found {} URLs", sitemap.urls.len());
/// # Ok(())
/// # }
/// ```
pub async fn fetch_sitemap(sitemap_url: &str) -> Result<Sitemap> {
    let xml = fetch_html(sitemap_url).await?;
    parse_sitemap(&xml)
}

/// Parses XML sitemap content into a Sitemap struct.
///
/// # Errors
///
/// Returns an error if the XML is malformed or no URLs are found.
fn parse_sitemap(xml: &str) -> Result<Sitemap> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut urls = Vec::new();
    let mut current_url: Option<String> = None;
    let mut current_lastmod: Option<String> = None;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                match e.name().as_ref() {
                    b"url" => {
                        // Start of a new URL entry
                        current_url = None;
                        current_lastmod = None;
                    }
                    b"loc" => {
                        // Read the loc text
                        if let Ok(Event::Text(text)) = reader.read_event_into(&mut buf) {
                            current_url = Some(
                                text.unescape()
                                    .map_err(|e| {
                                        LlmsGenError::SitemapError(format!("Invalid XML: {}", e))
                                    })?
                                    .to_string(),
                            );
                        }
                    }
                    b"lastmod" => {
                        // Read the lastmod text
                        if let Ok(Event::Text(text)) = reader.read_event_into(&mut buf) {
                            current_lastmod = Some(
                                text.unescape()
                                    .map_err(|e| {
                                        LlmsGenError::SitemapError(format!("Invalid XML: {}", e))
                                    })?
                                    .to_string(),
                            );
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"url" {
                    // End of URL entry, save it
                    if let Some(loc) = current_url.take() {
                        urls.push(SitemapUrl {
                            loc,
                            lastmod: current_lastmod.take(),
                        });
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(LlmsGenError::SitemapError(format!(
                    "XML parsing error: {}",
                    e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    if urls.is_empty() {
        return Err(LlmsGenError::SitemapError(
            "No URLs found in sitemap".to_string(),
        ));
    }

    Ok(Sitemap { urls })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sitemap() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/page1</loc>
    <lastmod>2024-01-01</lastmod>
  </url>
  <url>
    <loc>https://example.com/page2</loc>
  </url>
</urlset>"#;

        let sitemap = parse_sitemap(xml).unwrap();
        assert_eq!(sitemap.urls.len(), 2);
        assert_eq!(sitemap.urls[0].loc, "https://example.com/page1");
        assert_eq!(sitemap.urls[0].lastmod, Some("2024-01-01".to_string()));
        assert_eq!(sitemap.urls[1].loc, "https://example.com/page2");
        assert_eq!(sitemap.urls[1].lastmod, None);
    }

    #[test]
    fn test_parse_sitemap_empty() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
</urlset>"#;

        let result = parse_sitemap(xml);
        assert!(result.is_err());
    }

    #[test]
    fn test_sitemap_sites() {
        let sitemap = Sitemap {
            urls: vec![
                SitemapUrl {
                    loc: "https://example.com/page1".to_string(),
                    lastmod: None,
                },
                SitemapUrl {
                    loc: "https://example.com/page2".to_string(),
                    lastmod: None,
                },
            ],
        };

        let sites = sitemap.sites();
        assert_eq!(sites.len(), 2);
        assert_eq!(sites[0], "https://example.com/page1");
        assert_eq!(sites[1], "https://example.com/page2");
    }
}
