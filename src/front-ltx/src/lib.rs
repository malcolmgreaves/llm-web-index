use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Document, HtmlElement, HtmlInputElement, Request, RequestInit, RequestMode, Response, console,
};

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

#[derive(Debug, Deserialize, Serialize)]
struct JobIdPayload {
    job_id: String,
}

#[derive(Debug, Deserialize)]
struct JobState {
    job_id: String,
    url: String,
    status: String,
    kind: String,
    llms_txt: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
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

    show_page(&document, Page::Main)?;

    Ok(())
}

// ============================================================================
// Navigation
// ============================================================================

fn show_page(document: &Document, page: Page) -> Result<(), JsValue> {
    let body = document.body().expect("document should have a body");
    body.set_inner_html("");

    let container = document.create_element("div")?;
    container.set_id("wasm-container");

    match page {
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

fn create_get_llmstxt_page(
    document: &Document,
    container: &web_sys::Element,
) -> Result<(), JsValue> {
    container.append_child(&create_back_button(document)?)?;

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
                    console::error_1(&format!("Error: {:?}", e).into());
                    display_text_result(&format!("Error: {:?}", e));
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

fn create_generate_or_update_page(
    document: &Document,
    container: &web_sys::Element,
) -> Result<(), JsValue> {
    container.append_child(&create_back_button(document)?)?;

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
    container.append_child(&create_back_button(document)?)?;

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

fn create_list_in_progress_page(
    document: &Document,
    container: &web_sys::Element,
) -> Result<(), JsValue> {
    container.append_child(&create_back_button(document)?)?;

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

fn create_inspect_job_page(
    document: &Document,
    container: &web_sys::Element,
) -> Result<(), JsValue> {
    container.append_child(&create_back_button(document)?)?;

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
    let payload = UrlPayload {
        url: url.to_string(),
    };
    let payload_json = serde_json::to_string(&payload).unwrap();

    api_request("/api/llm_txt", "GET", Some(&payload_json)).await
}

async fn put_llm_txt(url: &str) -> Result<String, JsValue> {
    let payload = UrlPayload {
        url: url.to_string(),
    };
    let payload_json = serde_json::to_string(&payload).unwrap();

    let response: serde_json::Value =
        api_request("/api/llm_txt", "PUT", Some(&payload_json)).await?;
    Ok(serde_json::to_string_pretty(&response).unwrap())
}

async fn fetch_list() -> Result<LlmsTxtListResponse, JsValue> {
    api_request("/api/list", "GET", None).await
}

async fn fetch_in_progress_jobs() -> Result<Vec<JobState>, JsValue> {
    api_request("/api/jobs/in_progress", "GET", None).await
}

async fn fetch_job(job_id: &str) -> Result<JobState, JsValue> {
    let payload = JobIdPayload {
        job_id: job_id.to_string(),
    };
    let payload_json = serde_json::to_string(&payload).unwrap();

    api_request("/api/job", "GET", Some(&payload_json)).await
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

    let json = JsFuture::from(resp.json()?).await?;
    let data: T = serde_wasm_bindgen::from_value(json)?;

    Ok(data)
}

// ============================================================================
// Display Helpers
// ============================================================================

fn display_text_result(text: &str) {
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("should have a document on window");

    let results_div = document
        .get_element_by_id("results")
        .expect("results div should exist");

    let pre = document.create_element("pre").unwrap();
    pre.set_class_name("result-text");
    pre.set_text_content(Some(text));

    results_div.set_inner_html("");
    results_div.append_child(&pre).unwrap();
}

fn display_list_results(data: &LlmsTxtListResponse) {
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("should have a document on window");

    let results_div = document
        .get_element_by_id("results")
        .expect("results div should exist");

    results_div.set_inner_html("");

    for item in &data.items {
        let item_div = document.create_element("div").unwrap();
        item_div.set_class_name("list-item");

        let url_heading = document.create_element("h3").unwrap();
        url_heading.set_text_content(Some(&item.url));
        item_div.append_child(&url_heading).unwrap();

        let content_pre = document.create_element("pre").unwrap();
        content_pre.set_class_name("llm-txt-content");
        content_pre.set_text_content(Some(&item.llm_txt));
        item_div.append_child(&content_pre).unwrap();

        results_div.append_child(&item_div).unwrap();
    }
}

fn display_jobs_results(jobs: &[JobState]) {
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("should have a document on window");

    let results_div = document
        .get_element_by_id("results")
        .expect("results div should exist");

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

    let results_div = document
        .get_element_by_id("results")
        .expect("results div should exist");

    results_div.set_inner_html("");

    let job_div = document.create_element("div").unwrap();
    job_div.set_class_name("job-details");

    let mut job_info = format!(
        "Job ID: {}\nURL: {}\nStatus: {}\nKind: {}",
        job.job_id, job.url, job.status, job.kind
    );

    if let Some(ref llms_txt) = job.llms_txt {
        job_info.push_str(&format!("\n\nLLMs.txt Content:\n{}", llms_txt));
    }

    let job_pre = document.create_element("pre").unwrap();
    job_pre.set_text_content(Some(&job_info));
    job_div.append_child(&job_pre).unwrap();

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
