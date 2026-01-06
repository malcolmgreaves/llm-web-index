use crate::Error;
use url::Url;

pub fn is_valid_url(url: &str) -> Result<Url, Error> {
    let valid_url = Url::parse(url)?;
    Ok(valid_url)
}

pub async fn download_html(url: &Url) -> Result<String, Error> {
    let response = reqwest::get(url.as_str()).await?;
    let text_body = response.text().await?;
    Ok(text_body)
}
