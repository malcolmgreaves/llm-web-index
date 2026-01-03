use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Document, HtmlElement, Request, RequestInit, RequestMode, Response, console};

#[derive(Debug, Deserialize)]
struct LlmsTxtListItem {
    url: String,
    llm_txt: String,
}

#[derive(Debug, Deserialize)]
struct LlmsTxtListResponse {
    items: Vec<LlmsTxtListItem>,
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console::log_1(&"WASM module initialized!".into());

    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("should have a document on window");

    create_ui(&document)?;

    Ok(())
}

fn create_ui(document: &Document) -> Result<(), JsValue> {
    let body = document.body().expect("document should have a body");

    let container = document.create_element("div")?;
    container.set_id("wasm-container");

    let heading = document.create_element("h1")?;
    heading.set_text_content(Some("LLMs.txt List"));

    let button = document.create_element("button")?;
    button.set_text_content(Some("Load LLMs.txt List"));
    button.set_id("load-button");

    let results_div = document.create_element("div")?;
    results_div.set_id("results");

    let closure = Closure::wrap(Box::new(move || {
        console::log_1(&"Button clicked!".into());
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_list().await {
                Ok(data) => display_results(&data),
                Err(e) => {
                    console::error_1(&format!("Error fetching list: {:?}", e).into());
                    show_error_dialog(&format!("Error: {:?}", e));
                }
            }
        });
    }) as Box<dyn Fn()>);

    button
        .dyn_ref::<HtmlElement>()
        .expect("button should be an HtmlElement")
        .set_onclick(Some(closure.as_ref().unchecked_ref()));

    closure.forget();

    container.append_child(&heading)?;
    container.append_child(&button)?;
    container.append_child(&results_div)?;
    body.append_child(&container)?;

    Ok(())
}

async fn fetch_list() -> Result<LlmsTxtListResponse, JsValue> {
    let window = web_sys::window().expect("no global window exists");

    let opts = &mut RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init("/api/list", opts)?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;

    let data: LlmsTxtListResponse = serde_wasm_bindgen::from_value(json)?;

    Ok(data)
}

fn display_results(data: &LlmsTxtListResponse) {
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("should have a document on window");

    let results_div = document
        .get_element_by_id("results")
        .expect("results div should exist");

    results_div.set_inner_html("");

    if data.items.is_empty() {
        let message = document.create_element("p").unwrap();
        message.set_text_content(Some("No llms.txt results exist!"));
        results_div.append_child(&message).unwrap();
        return;
    }

    let dialog_html = create_dialog_html(data);
    results_div.set_inner_html(&dialog_html);
}

fn create_dialog_html(data: &LlmsTxtListResponse) -> String {
    let mut html = String::from(r#"<div class="dialog-overlay"><div class="dialog-content">"#);
    html.push_str("<h2>LLMs.txt List Results</h2>");
    html.push_str(&format!("<p>Found {} items:</p>", data.items.len()));
    html.push_str("<div class='items-container'>");

    for item in &data.items {
        html.push_str("<div class='item'>");
        html.push_str(&format!("<h3>{}</h3>", escape_html(&item.url)));
        html.push_str("<pre>");
        html.push_str(&escape_html(&item.llm_txt));
        html.push_str("</pre>");
        html.push_str("</div>");
    }

    html.push_str("</div>");
    html.push_str(
        r#"<button onclick="document.getElementById('results').innerHTML = ''">Close</button>"#,
    );
    html.push_str("</div></div>");
    html
}

fn show_error_dialog(message: &str) {
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("should have a document on window");

    let results_div = document
        .get_element_by_id("results")
        .expect("results div should exist");

    let html = format!(
        r#"<div class="dialog-overlay"><div class="dialog-content error">
            <h2>Error</h2>
            <p>{}</p>
            <button onclick="document.getElementById('results').innerHTML = ''">Close</button>
        </div></div>"#,
        escape_html(message)
    );

    results_div.set_inner_html(&html);
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
