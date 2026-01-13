mod auth;

use pulldown_cmark::{Parser, html};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Document, HtmlElement, HtmlInputElement, Request, RequestInit, RequestMode, Response, console};

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Deserialize)]
struct LlmsTxtListItem {
    url: String,
    llm_txt: String,
}

#[derive(Debug, Deserialize)]
struct LlmsTxtListResponse {
    items: Vec<LlmsTxtListItem>,
}

#[derive(Debug, Deserialize, Serialize)]
struct UrlPayload {
    url: String,
}

#[derive(Debug, Deserialize)]
struct LlmTxtResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
struct JobState {
    job_id: String,
    url: String,
    status: String,
    kind: String,
    llms_txt: Option<String>,
    error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Login,
    Main,
    GetLlmsTxt,
    GenerateOrUpdate,
    ListAll,
    ListInProgress,
    InspectJob,
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console::log_1(&"WASM module initialized!".into());

    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("should have a document on window");

    // Check authentication status before deciding which page to show
    let document_clone = document.clone();
    spawn_local(async move {
        match auth::check_auth_status().await {
            Ok(auth_status) => {
                if auth_status.auth_enabled && !auth_status.authenticated {
                    console::log_1(&"Auth required, showing login page".into());
                    show_page(&document_clone, Page::Login).ok();
                } else {
                    console::log_1(&"Auth not required or already authenticated, showing main page".into());
                    show_page(&document_clone, Page::Main).ok();
                }
            }
            Err(e) => {
                console::log_1(&format!("Auth check failed: {:?}, showing main page", e).into());
                show_page(&document_clone, Page::Main).ok();
            }
        }
    });

    Ok(())
}

// ============================================================================
// Navigation
// ============================================================================

pub fn show_page(document: &Document, page: Page) -> Result<(), JsValue> {
    let body = document.body().expect("document should have a body");
    body.set_inner_html("");

    let container = document.create_element("div")?;
    container.set_id("wasm-container");

    match page {
        Page::Login => auth::create_login_page(document, &container)?,
        Page::Main => create_main_page(document, &container)?,
        Page::GetLlmsTxt => create_get_llmstxt_page(document, &container)?,
        Page::GenerateOrUpdate => create_generate_or_update_page(document, &container)?,
        Page::ListAll => create_list_all_page(document, &container)?,
        Page::ListInProgress => create_list_in_progress_page(document, &container)?,
        Page::InspectJob => create_inspect_job_page(document, &container)?,
    }

    body.append_child(&container)?;
    Ok(())
}

fn create_back_button(document: &Document) -> Result<web_sys::Element, JsValue> {
    let back_btn = document.create_element("button")?;
    back_btn.set_class_name("back-button");
    back_btn.set_text_content(Some("← Back"));

    let closure = Closure::wrap(Box::new(move || {
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document on window");
        show_page(&document, Page::Main).ok();
    }) as Box<dyn Fn()>);

    back_btn
        .dyn_ref::<HtmlElement>()
        .expect("button should be an HtmlElement")
        .set_onclick(Some(closure.as_ref().unchecked_ref()));

    closure.forget();

    Ok(back_btn)
}

// ============================================================================
// Page 0: Main Menu
// ============================================================================

fn create_main_page(document: &Document, container: &web_sys::Element) -> Result<(), JsValue> {
    let heading = document.create_element("h1")?;
    heading.set_text_content(Some("LLMs.txt Manager"));
    container.append_child(&heading)?;

    let pages = [
        (Page::GetLlmsTxt, "See an llms.txt for a website"),
        (
            Page::GenerateOrUpdate,
            "Generate a new or update an llms.txt for a website",
        ),
        (Page::ListAll, "List all up-to-date llms.txts"),
        (Page::ListInProgress, "List all in-progress jobs"),
        (Page::InspectJob, "Inspect an in-progress job"),
    ];

    for (page, label) in &pages {
        let button = document.create_element("button")?;
        button.set_text_content(Some(label));
        button.set_class_name("menu-button");

        let page_copy = *page;
        let closure = Closure::wrap(Box::new(move || {
            let window = web_sys::window().expect("no global window exists");
            let document = window.document().expect("should have a document on window");
            show_page(&document, page_copy).ok();
        }) as Box<dyn Fn()>);

        button
            .dyn_ref::<HtmlElement>()
            .expect("button should be an HtmlElement")
            .set_onclick(Some(closure.as_ref().unchecked_ref()));

        closure.forget();

        container.append_child(&button)?;
    }

    Ok(())
}

// ============================================================================
// Page 1: Get LLMs.txt for a Website
// ============================================================================

fn create_get_llmstxt_page(document: &Document, container: &web_sys::Element) -> Result<(), JsValue> {
    container.append_child(&create_back_button(document)?.into())?;

    let heading = document.create_element("h1")?;
    heading.set_text_content(Some("Get LLMs.txt for a Website"));
    container.append_child(&heading)?;

    let input_container = document.create_element("div")?;
    input_container.set_class_name("input-group");

    let input = document.create_element("input")?;
    input.set_attribute("type", "text")?;
    input.set_attribute("placeholder", "Enter website URL...")?;
    input.set_id("url-input");

    let search_btn = document.create_element("button")?;
    search_btn.set_text_content(Some("Search"));

    input_container.append_child(&input)?;
    input_container.append_child(&search_btn)?;
    container.append_child(&input_container)?;

    let results_div = document.create_element("div")?;
    results_div.set_id("results");
    results_div.set_class_name("results");
    container.append_child(&results_div)?;

    let closure = Closure::wrap(Box::new(move || {
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document on window");

        let input = document
            .get_element_by_id("url-input")
            .expect("input should exist")
            .dyn_into::<HtmlInputElement>()
            .expect("should be input element");

        let url = input.value().trim().to_string();

        if url.is_empty() {
            show_error_modal(&document, "URL cannot be empty");
            return;
        }

        if !is_valid_url(&url) {
            show_error_modal(&document, "Please enter a valid URL");
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            match fetch_llm_txt(&url).await {
                Ok(data) => display_text_result(&data.content),
                Err(e) => {
                    console::error_1(&format!("Could not retrieve llms.txt file due to: {:?}", e).into());
                    display_text_result(&format!("Could not retrieve llms.txt file due to: {:?}", e));
                }
            }
        });
    }) as Box<dyn Fn()>);

    search_btn
        .dyn_ref::<HtmlElement>()
        .expect("button should be an HtmlElement")
        .set_onclick(Some(closure.as_ref().unchecked_ref()));

    closure.forget();

    Ok(())
}

// ============================================================================
// Page 2: Generate or Update LLMs.txt
// ============================================================================

fn create_generate_or_update_page(document: &Document, container: &web_sys::Element) -> Result<(), JsValue> {
    container.append_child(&create_back_button(document)?.into())?;

    let heading = document.create_element("h1")?;
    heading.set_text_content(Some("Generate or Update LLMs.txt"));
    container.append_child(&heading)?;

    let input_container = document.create_element("div")?;
    input_container.set_class_name("input-group");

    let input = document.create_element("input")?;
    input.set_attribute("type", "text")?;
    input.set_attribute("placeholder", "Enter website URL...")?;
    input.set_id("url-input");

    let generate_btn = document.create_element("button")?;
    generate_btn.set_text_content(Some("Generate or Update"));

    input_container.append_child(&input)?;
    input_container.append_child(&generate_btn)?;
    container.append_child(&input_container)?;

    let results_div = document.create_element("div")?;
    results_div.set_id("results");
    results_div.set_class_name("results");
    container.append_child(&results_div)?;

    let closure = Closure::wrap(Box::new(move || {
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document on window");

        let input = document
            .get_element_by_id("url-input")
            .expect("input should exist")
            .dyn_into::<HtmlInputElement>()
            .expect("should be input element");

        let url = input.value().trim().to_string();

        if url.is_empty() {
            show_error_modal(&document, "URL cannot be empty");
            return;
        }

        if !is_valid_url(&url) {
            show_error_modal(&document, "Please enter a valid URL");
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            match put_llm_txt(&url).await {
                Ok(response_text) => display_text_result(&response_text),
                Err(e) => {
                    console::error_1(&format!("Error: {:?}", e).into());
                    display_text_result(&format!("Error: {:?}", e));
                }
            }
        });
    }) as Box<dyn Fn()>);

    generate_btn
        .dyn_ref::<HtmlElement>()
        .expect("button should be an HtmlElement")
        .set_onclick(Some(closure.as_ref().unchecked_ref()));

    closure.forget();

    Ok(())
}

// ============================================================================
// Page 3: List All Up-to-Date LLMs.txts
// ============================================================================

fn create_list_all_page(document: &Document, container: &web_sys::Element) -> Result<(), JsValue> {
    container.append_child(&create_back_button(document)?.into())?;

    let heading = document.create_element("h1")?;
    heading.set_text_content(Some("All Up-to-Date LLMs.txts"));
    container.append_child(&heading)?;

    let results_div = document.create_element("div")?;
    results_div.set_id("results");
    results_div.set_class_name("results");
    container.append_child(&results_div)?;

    wasm_bindgen_futures::spawn_local(async move {
        match fetch_list().await {
            Ok(data) => {
                if data.items.is_empty() {
                    display_text_result("No llms.txt results exist!");
                } else {
                    display_list_results(&data);
                }
            }
            Err(e) => {
                console::error_1(&format!("Error: {:?}", e).into());
                display_text_result(&format!("Error: {:?}", e));
            }
        }
    });

    Ok(())
}

// ============================================================================
// Page 4: List All In-Progress Jobs
// ============================================================================

fn create_list_in_progress_page(document: &Document, container: &web_sys::Element) -> Result<(), JsValue> {
    container.append_child(&create_back_button(document)?.into())?;

    let heading = document.create_element("h1")?;
    heading.set_text_content(Some("All In-Progress Jobs"));
    container.append_child(&heading)?;

    let results_div = document.create_element("div")?;
    results_div.set_id("results");
    results_div.set_class_name("results");
    container.append_child(&results_div)?;

    wasm_bindgen_futures::spawn_local(async move {
        match fetch_in_progress_jobs().await {
            Ok(jobs) => {
                if jobs.is_empty() {
                    display_text_result("No in-progress jobs.");
                } else {
                    display_jobs_results(&jobs);
                }
            }
            Err(e) => {
                console::error_1(&format!("Error: {:?}", e).into());
                display_text_result(&format!("Error: {:?}", e));
            }
        }
    });

    Ok(())
}

// ============================================================================
// Page 5: Inspect Job by UUID
// ============================================================================

fn create_inspect_job_page(document: &Document, container: &web_sys::Element) -> Result<(), JsValue> {
    container.append_child(&create_back_button(document)?.into())?;

    let heading = document.create_element("h1")?;
    heading.set_text_content(Some("Inspect Job"));
    container.append_child(&heading)?;

    let input_container = document.create_element("div")?;
    input_container.set_class_name("input-group");

    let input = document.create_element("input")?;
    input.set_attribute("type", "text")?;
    input.set_attribute("placeholder", "Enter job UUID...")?;
    input.set_id("job-id-input");

    let inspect_btn = document.create_element("button")?;
    inspect_btn.set_text_content(Some("Inspect"));

    input_container.append_child(&input)?;
    input_container.append_child(&inspect_btn)?;
    container.append_child(&input_container)?;

    let results_div = document.create_element("div")?;
    results_div.set_id("results");
    results_div.set_class_name("results");
    container.append_child(&results_div)?;

    let closure = Closure::wrap(Box::new(move || {
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document on window");

        let input = document
            .get_element_by_id("job-id-input")
            .expect("input should exist")
            .dyn_into::<HtmlInputElement>()
            .expect("should be input element");

        let job_id = input.value().trim().to_string();

        if job_id.is_empty() {
            show_error_modal(&document, "Job ID cannot be empty");
            return;
        }

        if !is_valid_uuid(&job_id) {
            show_error_modal(&document, "Please enter a valid UUID v4");
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            match fetch_job(&job_id).await {
                Ok(job) => display_job_details(&job),
                Err(e) => {
                    console::error_1(&format!("Error: {:?}", e).into());
                    display_text_result(&format!("Error: {:?}", e));
                }
            }
        });
    }) as Box<dyn Fn()>);

    inspect_btn
        .dyn_ref::<HtmlElement>()
        .expect("button should be an HtmlElement")
        .set_onclick(Some(closure.as_ref().unchecked_ref()));

    closure.forget();

    Ok(())
}

// ============================================================================
// API Calls
// ============================================================================

async fn fetch_llm_txt(url: &str) -> Result<LlmTxtResponse, JsValue> {
    let encoded_url = js_sys::encode_uri_component(url);
    let endpoint = format!("/api/llm_txt?url={}", encoded_url);

    api_request(&endpoint, "GET", None).await
}

async fn put_llm_txt(url: &str) -> Result<String, JsValue> {
    let payload = UrlPayload { url: url.to_string() };
    let payload_json = serde_json::to_string(&payload).unwrap();

    let response: serde_json::Value = api_request("/api/llm_txt", "PUT", Some(&payload_json)).await?;
    Ok(serde_json::to_string_pretty(&response).unwrap())
}

async fn fetch_list() -> Result<LlmsTxtListResponse, JsValue> {
    api_request("/api/list", "GET", None).await
}

async fn fetch_in_progress_jobs() -> Result<Vec<JobState>, JsValue> {
    api_request("/api/jobs/in_progress", "GET", None).await
}

async fn fetch_job(job_id: &str) -> Result<JobState, JsValue> {
    let endpoint = format!("/api/job?job_id={}", job_id);

    api_request(&endpoint, "GET", None).await
}

async fn api_request<T: for<'de> Deserialize<'de>>(
    endpoint: &str,
    method: &str,
    body: Option<&str>,
) -> Result<T, JsValue> {
    let window = web_sys::window().expect("no global window exists");

    let opts = &mut RequestInit::new();
    opts.set_method(method);
    opts.set_mode(RequestMode::Cors);

    if let Some(body_str) = body {
        opts.set_body(&JsValue::from_str(body_str));
    }

    let request = Request::new_with_str_and_init(endpoint, opts)?;
    request.headers().set("Content-Type", "application/json")?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    // Check if the response status is OK (200-299)
    if !resp.ok() {
        let text = JsFuture::from(resp.text()?).await?;
        let error_text = text.as_string().unwrap_or_else(|| "Unknown error".to_string());
        return Err(JsValue::from_str(&error_text));
    }

    let json = JsFuture::from(resp.json()?).await?;
    let data: T = serde_wasm_bindgen::from_value(json)?;

    Ok(data)
}

// ============================================================================
// Display Helpers
// ============================================================================

/// Renders markdown content to HTML with plain text fallback.
///
/// This function parses the input as markdown and converts it to HTML.
/// The pulldown-cmark library is designed to be robust and handles any markdown input
/// gracefully, so this function should not fail under normal circumstances.
///
/// As a safety measure, if the rendered output is empty when the input is not,
/// the function falls back to displaying the content as plain text in a `<pre>` element.
///
/// # Arguments
/// * `content` - The markdown content to render
///
/// # Returns
/// HTML string with rendered content. Either markdown-rendered HTML or plain text fallback.
fn render_markdown_with_fallback(content: &str) -> String {
    // Parse and render markdown
    let parser = Parser::new(content);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // Safety check: if rendering produced empty output from non-empty input, use fallback
    if html_output.trim().is_empty() && !content.trim().is_empty() {
        console::log_1(&"Markdown rendering produced empty output, falling back to plain text".into());
        return format!(
            r#"<pre class="result-text fallback-text">{}</pre>"#,
            html_escape(content)
        );
    }

    // Return successfully rendered markdown
    format!(r#"<div class="markdown-content">{}</div>"#, html_output)
}

/// Escapes HTML special characters to prevent XSS and rendering issues.
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Creates a toggle button DOM element for switching between markdown and plaintext views.
///
/// # Arguments
/// * `document` - The document to create elements in
/// * `id_suffix` - A unique suffix for element IDs
///
/// # Returns
/// A toggle button element
fn create_view_toggle(document: &Document, id_suffix: &str) -> Result<web_sys::Element, JsValue> {
    let toggle = document.create_element("div")?;
    toggle.set_class_name("view-toggle");
    toggle.set_id(&format!("toggle-{}", id_suffix));
    toggle.set_text_content(Some("Show plaintext"));

    let id_suffix_clone = id_suffix.to_string();
    let closure = Closure::wrap(Box::new(move || {
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document on window");

        let toggle = document
            .get_element_by_id(&format!("toggle-{}", id_suffix_clone))
            .unwrap();
        let markdown = document
            .get_element_by_id(&format!("markdown-{}", id_suffix_clone))
            .unwrap();
        let plaintext = document
            .get_element_by_id(&format!("plaintext-{}", id_suffix_clone))
            .unwrap();

        if markdown.get_attribute("style").unwrap_or_default().contains("none") {
            markdown.set_attribute("style", "display: block;").unwrap();
            plaintext.set_attribute("style", "display: none;").unwrap();
            toggle.set_text_content(Some("Show plaintext"));
        } else {
            markdown.set_attribute("style", "display: none;").unwrap();
            plaintext.set_attribute("style", "display: block;").unwrap();
            toggle.set_text_content(Some("Show markdown"));
        }
    }) as Box<dyn Fn()>);

    toggle
        .dyn_ref::<HtmlElement>()
        .expect("toggle should be an HtmlElement")
        .set_onclick(Some(closure.as_ref().unchecked_ref()));

    closure.forget();

    Ok(toggle)
}

/// Renders content with markdown and plaintext views (without the toggle button).
///
/// Creates content divs for both markdown and plaintext:
/// - Markdown-rendered content (visible by default)
/// - Plaintext content (hidden by default)
///
/// # Arguments
/// * `content` - The content to render
/// * `id_suffix` - A unique suffix for element IDs
///
/// # Returns
/// HTML string with both content views
fn render_content_views(content: &str, id_suffix: &str) -> String {
    let markdown_html = render_markdown_with_fallback(content);
    let plaintext_html = format!(r#"<pre class="plaintext-content">{}</pre>"#, html_escape(content));

    format!(
        r#"<div id="markdown-{}">{}</div>
        <div id="plaintext-{}" style="display: none;">{}</div>"#,
        id_suffix, markdown_html, id_suffix, plaintext_html
    )
}

fn display_text_result(text: &str) {
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("should have a document on window");

    let results_div = document.get_element_by_id("results").expect("results div should exist");

    // Clear previous content
    results_div.set_inner_html("");

    // Create and append toggle button
    let toggle = create_view_toggle(&document, "text-result").unwrap();
    results_div.append_child(&toggle).unwrap();

    // Create content container and set HTML with both views
    let content_container = document.create_element("div").unwrap();
    let content_html = render_content_views(text, "text-result");
    content_container.set_inner_html(&content_html);
    results_div.append_child(&content_container).unwrap();
}

fn display_list_results(data: &LlmsTxtListResponse) {
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("should have a document on window");

    let results_div = document.get_element_by_id("results").expect("results div should exist");

    results_div.set_inner_html("");

    for (index, item) in data.items.iter().enumerate() {
        let item_div = document.create_element("div").unwrap();
        item_div.set_class_name("list-item");

        let url_heading = document.create_element("h3").unwrap();
        url_heading.set_text_content(Some(&item.url));
        item_div.append_child(&url_heading).unwrap();

        let lines: Vec<&str> = item.llm_txt.lines().collect();
        let total_lines = lines.len();
        let preview_lines = 20;

        if total_lines > preview_lines {
            let preview_content: String = lines.iter().take(preview_lines).copied().collect::<Vec<_>>().join("\n");
            let full_content = item.llm_txt.clone();

            // Add toggle for preview
            let toggle_preview = create_view_toggle(&document, &format!("list-preview-{}", index)).unwrap();
            toggle_preview.set_id(&format!("toggle-preview-{}", index));
            item_div.append_child(&toggle_preview).unwrap();

            // Add toggle for full content (hidden by default)
            let toggle_full = create_view_toggle(&document, &format!("list-full-{}", index)).unwrap();
            toggle_full.set_id(&format!("toggle-full-{}", index));
            toggle_full.set_attribute("style", "display: none;").unwrap();
            item_div.append_child(&toggle_full).unwrap();

            // Render preview content
            let preview_div = document.create_element("div").unwrap();
            preview_div.set_class_name("llm-txt-content");
            preview_div.set_id(&format!("preview-{}", index));
            let preview_html = render_content_views(&preview_content, &format!("list-preview-{}", index));
            preview_div.set_inner_html(&preview_html);
            item_div.append_child(&preview_div).unwrap();

            // Render full content (hidden by default)
            let full_div = document.create_element("div").unwrap();
            full_div.set_class_name("llm-txt-content");
            full_div.set_id(&format!("full-{}", index));
            full_div.set_attribute("style", "display: none;").unwrap();
            let full_html = render_content_views(&full_content, &format!("list-full-{}", index));
            full_div.set_inner_html(&full_html);
            item_div.append_child(&full_div).unwrap();

            let expand_link = document.create_element("div").unwrap();
            expand_link.set_class_name("expand-link");
            expand_link.set_id(&format!("expand-{}", index));
            expand_link.set_text_content(Some("expand to see more"));
            item_div.append_child(&expand_link).unwrap();

            let collapse_link = document.create_element("div").unwrap();
            collapse_link.set_class_name("collapse-link");
            collapse_link.set_id(&format!("collapse-{}", index));
            collapse_link.set_attribute("style", "display: none;").unwrap();
            collapse_link.set_text_content(Some("collapse"));
            item_div.append_child(&collapse_link).unwrap();

            let expand_closure = {
                let document = document.clone();
                let idx = index;
                Closure::wrap(Box::new(move || {
                    let preview = document.get_element_by_id(&format!("preview-{}", idx)).unwrap();
                    let full = document.get_element_by_id(&format!("full-{}", idx)).unwrap();
                    let expand = document.get_element_by_id(&format!("expand-{}", idx)).unwrap();
                    let collapse = document.get_element_by_id(&format!("collapse-{}", idx)).unwrap();
                    let toggle_preview = document.get_element_by_id(&format!("toggle-preview-{}", idx)).unwrap();
                    let toggle_full = document.get_element_by_id(&format!("toggle-full-{}", idx)).unwrap();

                    preview.set_attribute("style", "display: none;").unwrap();
                    full.set_attribute("style", "display: block;").unwrap();
                    expand.set_attribute("style", "display: none;").unwrap();
                    collapse.set_attribute("style", "display: block;").unwrap();
                    toggle_preview.set_attribute("style", "display: none;").unwrap();
                    toggle_full.set_attribute("style", "display: inline-block;").unwrap();
                }) as Box<dyn Fn()>)
            };

            expand_link
                .dyn_ref::<HtmlElement>()
                .unwrap()
                .set_onclick(Some(expand_closure.as_ref().unchecked_ref()));
            expand_closure.forget();

            let collapse_closure = {
                let document = document.clone();
                let idx = index;
                Closure::wrap(Box::new(move || {
                    let preview = document.get_element_by_id(&format!("preview-{}", idx)).unwrap();
                    let full = document.get_element_by_id(&format!("full-{}", idx)).unwrap();
                    let expand = document.get_element_by_id(&format!("expand-{}", idx)).unwrap();
                    let collapse = document.get_element_by_id(&format!("collapse-{}", idx)).unwrap();
                    let toggle_preview = document.get_element_by_id(&format!("toggle-preview-{}", idx)).unwrap();
                    let toggle_full = document.get_element_by_id(&format!("toggle-full-{}", idx)).unwrap();

                    preview.set_attribute("style", "display: block;").unwrap();
                    full.set_attribute("style", "display: none;").unwrap();
                    expand.set_attribute("style", "display: block;").unwrap();
                    collapse.set_attribute("style", "display: none;").unwrap();
                    toggle_preview.set_attribute("style", "display: inline-block;").unwrap();
                    toggle_full.set_attribute("style", "display: none;").unwrap();
                }) as Box<dyn Fn()>)
            };

            collapse_link
                .dyn_ref::<HtmlElement>()
                .unwrap()
                .set_onclick(Some(collapse_closure.as_ref().unchecked_ref()));
            collapse_closure.forget();
        } else {
            // Add toggle for short content
            let toggle = create_view_toggle(&document, &format!("list-short-{}", index)).unwrap();
            item_div.append_child(&toggle).unwrap();

            // Render short content
            let content_div = document.create_element("div").unwrap();
            content_div.set_class_name("llm-txt-content");
            let content_html = render_content_views(&item.llm_txt, &format!("list-short-{}", index));
            content_div.set_inner_html(&content_html);
            item_div.append_child(&content_div).unwrap();
        }

        results_div.append_child(&item_div).unwrap();
    }
}

fn display_jobs_results(jobs: &[JobState]) {
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("should have a document on window");

    let results_div = document.get_element_by_id("results").expect("results div should exist");

    results_div.set_inner_html("");

    for job in jobs {
        let job_div = document.create_element("div").unwrap();
        job_div.set_class_name("job-item");

        let job_info = format!(
            "Job ID: {}\nURL: {}\nStatus: {}\nKind: {}",
            job.job_id, job.url, job.status, job.kind
        );

        let job_pre = document.create_element("pre").unwrap();
        job_pre.set_text_content(Some(&job_info));
        job_div.append_child(&job_pre).unwrap();

        results_div.append_child(&job_div).unwrap();
    }
}

fn display_job_details(job: &JobState) {
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("should have a document on window");

    let results_div = document.get_element_by_id("results").expect("results div should exist");

    results_div.set_inner_html("");

    let job_div = document.create_element("div").unwrap();
    job_div.set_class_name("job-details");

    let job_info = format!(
        "Job ID: {}\nURL: {}\nStatus: {}\nKind: {}",
        job.job_id, job.url, job.status, job.kind
    );

    // Display job metadata as plain text
    let job_pre = document.create_element("pre").unwrap();
    job_pre.set_text_content(Some(&job_info));
    job_div.append_child(&job_pre).unwrap();

    // Display error message if the job failed
    if job.status == "Failure"
        && let Some(ref error_msg) = job.error_message
    {
        let error_heading = document.create_element("h3").unwrap();
        error_heading.set_text_content(Some("Error Details:"));
        job_div.append_child(&error_heading).unwrap();

        let error_pre = document.create_element("pre").unwrap();
        error_pre.set_class_name("error-message");
        error_pre.set_text_content(Some(error_msg));
        job_div.append_child(&error_pre).unwrap();
    }

    // Render LLMs.txt content with toggle between markdown and plaintext
    if let Some(ref llms_txt) = job.llms_txt {
        let content_heading = document.create_element("h3").unwrap();
        content_heading.set_text_content(Some("LLMs.txt Content:"));
        job_div.append_child(&content_heading).unwrap();

        // Add toggle under the heading
        let toggle = create_view_toggle(&document, "job-detail").unwrap();
        job_div.append_child(&toggle).unwrap();

        // Render content
        let content_div = document.create_element("div").unwrap();
        content_div.set_class_name("llm-txt-content");
        let content_html = render_content_views(llms_txt, "job-detail");
        content_div.set_inner_html(&content_html);
        job_div.append_child(&content_div).unwrap();
    }

    results_div.append_child(&job_div).unwrap();
}

fn show_error_modal(document: &Document, message: &str) {
    let body = document.body().expect("document should have a body");

    let modal = document.create_element("div").unwrap();
    modal.set_class_name("modal");

    let modal_content = document.create_element("div").unwrap();
    modal_content.set_class_name("modal-content");

    let error_heading = document.create_element("h2").unwrap();
    error_heading.set_text_content(Some("Error"));
    modal_content.append_child(&error_heading).unwrap();

    let error_message = document.create_element("p").unwrap();
    error_message.set_text_content(Some(message));
    modal_content.append_child(&error_message).unwrap();

    let close_btn = document.create_element("button").unwrap();
    close_btn.set_text_content(Some("Close"));

    let modal_clone = modal.clone();
    let closure = Closure::wrap(Box::new(move || {
        modal_clone.remove();
    }) as Box<dyn Fn()>);

    close_btn
        .dyn_ref::<HtmlElement>()
        .unwrap()
        .set_onclick(Some(closure.as_ref().unchecked_ref()));

    closure.forget();

    modal_content.append_child(&close_btn).unwrap();
    modal.append_child(&modal_content).unwrap();
    body.append_child(&modal).unwrap();
}

// ============================================================================
// Validation Helpers
// ============================================================================

fn is_valid_url(url: &str) -> bool {
    // Permissive URL validation
    url.starts_with("http://") || url.starts_with("https://")
}

fn is_valid_uuid(uuid: &str) -> bool {
    // UUID v4 validation (8-4-4-4-12 format)
    let parts: Vec<&str> = uuid.split('-').collect();
    if parts.len() != 5 {
        return false;
    }

    let expected_lengths = [8, 4, 4, 4, 12];
    for (i, part) in parts.iter().enumerate() {
        if part.len() != expected_lengths[i] {
            return false;
        }
        if !part.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
    }

    true
}
