use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Document, HtmlInputElement, Request, RequestInit, RequestMode, Response, console};

use crate::Page;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AuthCheckResponse {
    pub auth_enabled: bool,
    pub authenticated: bool,
}

#[derive(Debug, Serialize)]
struct LoginRequest {
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    #[allow(dead_code)]
    success: bool,
}

// ============================================================================
// API Functions
// ============================================================================

/// Check authentication status
pub async fn check_auth_status() -> Result<AuthCheckResponse, JsValue> {
    let window = web_sys::window().expect("no global window exists");

    let opts = &mut RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init("/api/auth/check", opts)?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    if !resp.ok() {
        return Err(JsValue::from_str("Failed to check auth status"));
    }

    let json = JsFuture::from(resp.json()?).await?;
    let data: AuthCheckResponse = serde_wasm_bindgen::from_value(json)?;

    Ok(data)
}

/// Login with password
async fn login(password: String) -> Result<LoginResponse, JsValue> {
    let window = web_sys::window().expect("no global window exists");

    let request_body = LoginRequest { password };
    let body_str = serde_json::to_string(&request_body)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize login request: {}", e)))?;

    let opts = &mut RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(&body_str));

    let request = Request::new_with_str_and_init("/api/auth/login", opts)?;
    request.headers().set("Content-Type", "application/json")?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    if !resp.ok() {
        let text = JsFuture::from(resp.text()?).await?;
        let error_text = text.as_string().unwrap_or_else(|| "Invalid credentials".to_string());
        return Err(JsValue::from_str(&error_text));
    }

    let json = JsFuture::from(resp.json()?).await?;
    let data: LoginResponse = serde_wasm_bindgen::from_value(json)?;

    Ok(data)
}

// ============================================================================
// UI Functions
// ============================================================================

/// Create login page
pub fn create_login_page(document: &Document, container: &web_sys::Element) -> Result<(), JsValue> {
    // Title
    let title = document.create_element("h1")?;
    title.set_text_content(Some("Authentication Required"));
    container.append_child(&title)?;

    // Password input group
    let input_group = document.create_element("div")?;
    input_group.set_class_name("input-group");

    let password_input = document.create_element("input")?.dyn_into::<HtmlInputElement>()?;
    password_input.set_type("password");
    password_input.set_placeholder("Enter password");
    password_input.set_id("password-input");

    let login_btn = document.create_element("button")?;
    login_btn.set_text_content(Some("Login"));
    login_btn.set_class_name("menu-button");

    input_group.append_child(&password_input)?;
    input_group.append_child(&login_btn)?;
    container.append_child(&input_group)?;

    // Error message div (initially hidden)
    let error_div = document.create_element("div")?;
    error_div.set_id("login-error");
    error_div.set_attribute("style", "color: #ff6b6b; margin-top: 1rem; display: none;")?;
    container.append_child(&error_div)?;

    // Login button click handler
    let document_clone = document.clone();
    let password_input_clone = password_input.clone();
    let closure = Closure::wrap(Box::new(move || {
        let document = document_clone.clone();
        let password_input = password_input_clone.clone();

        spawn_local(async move {
            let password = password_input.value();

            if password.is_empty() {
                show_login_error(&document, "Please enter a password");
                return;
            }

            // Disable button during login attempt
            if let Some(btn) = document.get_element_by_id("login-button") {
                btn.set_attribute("disabled", "true").ok();
            }

            match login(password).await {
                Ok(_) => {
                    console::log_1(&"Login successful".into());
                    // Navigate to main page
                    let window = web_sys::window().expect("no global window exists");
                    let document = window.document().expect("should have a document on window");
                    crate::show_page(&document, Page::Main).ok();
                }
                Err(e) => {
                    console::log_1(&format!("Login failed: {:?}", e).into());
                    show_login_error(&document, "Incorrect password");

                    // Re-enable button
                    if let Some(btn) = document.get_element_by_id("login-button") {
                        btn.remove_attribute("disabled").ok();
                    }

                    // Clear password field
                    password_input.set_value("");
                }
            }
        });
    }) as Box<dyn FnMut()>);

    login_btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
    closure.forget();

    // Enter key handler
    let document_clone2 = document.clone();
    let password_input_clone2 = password_input.clone();
    let closure2 = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        if event.key() == "Enter" {
            let document = document_clone2.clone();
            let password_input = password_input_clone2.clone();

            spawn_local(async move {
                let password = password_input.value();

                if password.is_empty() {
                    show_login_error(&document, "Please enter a password");
                    return;
                }

                match login(password).await {
                    Ok(_) => {
                        console::log_1(&"Login successful".into());
                        let window = web_sys::window().expect("no global window exists");
                        let document = window.document().expect("should have a document on window");
                        crate::show_page(&document, Page::Main).ok();
                    }
                    Err(e) => {
                        console::log_1(&format!("Login failed: {:?}", e).into());
                        show_login_error(&document, "Incorrect password");
                        password_input.set_value("");
                    }
                }
            });
        }
    }) as Box<dyn FnMut(_)>);

    password_input.add_event_listener_with_callback("keypress", closure2.as_ref().unchecked_ref())?;
    closure2.forget();

    Ok(())
}

fn show_login_error(document: &Document, message: &str) {
    if let Some(error_div) = document.get_element_by_id("login-error") {
        error_div.set_text_content(Some(message));
        error_div
            .set_attribute("style", "color: #ff6b6b; margin-top: 1rem; display: block;")
            .ok();
    }
}
