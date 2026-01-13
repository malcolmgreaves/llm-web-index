use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::{debug, warn};

use crate::errors::Error;

#[derive(Debug, Serialize)]
struct LoginRequest {
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    success: bool,
}

/// HTTP client with automatic authentication support
pub struct AuthenticatedClient {
    client: Client,
    api_base_url: String,
    password: Option<String>,
    cookie: Arc<Mutex<Option<String>>>,
}

impl AuthenticatedClient {
    /// Create a new authenticated client
    pub fn new(client: Client, api_base_url: String, password: Option<String>) -> Self {
        Self {
            client,
            api_base_url,
            password,
            cookie: Arc::new(Mutex::new(None)),
        }
    }

    /// Authenticate with the API server and store the session cookie
    pub async fn authenticate(&self) -> Result<(), Error> {
        let password = self
            .password
            .as_ref()
            .ok_or_else(|| Error::AuthError("No password configured for authentication".to_string()))?;

        let login_url = format!("{}/api/auth/login", self.api_base_url);
        let login_request = LoginRequest {
            password: password.clone(),
        };

        debug!("Authenticating with API server");

        let response = self
            .client
            .post(&login_url)
            .json(&login_request)
            .send()
            .await
            .map_err(|e| Error::HttpError(e))?;

        if !response.status().is_success() {
            return Err(Error::AuthError("Authentication failed".to_string()));
        }

        // Extract Set-Cookie header
        if let Some(set_cookie) = response.headers().get("set-cookie") {
            let cookie_value = set_cookie
                .to_str()
                .map_err(|_| Error::AuthError("Invalid cookie header".to_string()))?;

            // Extract just the cookie value (before the first semicolon)
            let cookie = cookie_value
                .split(';')
                .next()
                .ok_or_else(|| Error::AuthError("Invalid cookie format".to_string()))?
                .to_string();

            let mut cookie_guard = self
                .cookie
                .lock()
                .map_err(|_| Error::AuthError("Failed to lock cookie mutex".to_string()))?;
            *cookie_guard = Some(cookie);

            debug!("Authentication successful, cookie stored");
        } else {
            return Err(Error::AuthError("No cookie in response".to_string()));
        }

        Ok(())
    }

    /// Make a POST request with automatic authentication
    pub async fn post<T: Serialize>(&self, path: &str, json_body: &T) -> Result<Response, Error> {
        let url = format!("{}{}", self.api_base_url, path);

        // Try request with current cookie
        let mut request = self.client.post(&url).json(json_body);

        if let Ok(cookie_guard) = self.cookie.lock() {
            if let Some(cookie) = cookie_guard.as_ref() {
                request = request.header("Cookie", cookie);
            }
        }

        let response = request.send().await.map_err(|e| Error::HttpError(e))?;

        // If 401 and password is configured, try to re-authenticate
        if response.status() == StatusCode::UNAUTHORIZED && self.password.is_some() {
            warn!("Received 401, attempting to re-authenticate");

            self.authenticate().await?;

            // Retry request with new cookie
            let mut retry_request = self.client.post(&url).json(json_body);

            if let Ok(cookie_guard) = self.cookie.lock() {
                if let Some(cookie) = cookie_guard.as_ref() {
                    retry_request = retry_request.header("Cookie", cookie);
                }
            }

            let retry_response = retry_request.send().await.map_err(|e| Error::HttpError(e))?;

            return Ok(retry_response);
        }

        Ok(response)
    }

    /// Make a GET request with automatic authentication
    pub async fn get(&self, path: &str) -> Result<Response, Error> {
        let url = format!("{}{}", self.api_base_url, path);

        // Try request with current cookie
        let mut request = self.client.get(&url);

        if let Ok(cookie_guard) = self.cookie.lock() {
            if let Some(cookie) = cookie_guard.as_ref() {
                request = request.header("Cookie", cookie);
            }
        }

        let response = request.send().await.map_err(|e| Error::HttpError(e))?;

        // If 401 and password is configured, try to re-authenticate
        if response.status() == StatusCode::UNAUTHORIZED && self.password.is_some() {
            warn!("Received 401, attempting to re-authenticate");

            self.authenticate().await?;

            // Retry request with new cookie
            let mut retry_request = self.client.get(&url);

            if let Ok(cookie_guard) = self.cookie.lock() {
                if let Some(cookie) = cookie_guard.as_ref() {
                    retry_request = retry_request.header("Cookie", cookie);
                }
            }

            let retry_response = retry_request.send().await.map_err(|e| Error::HttpError(e))?;

            return Ok(retry_response);
        }

        Ok(response)
    }
}
