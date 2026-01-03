use wasm_bindgen::prelude::*;
use web_sys::{Document, HtmlElement, console};

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console::log_1(&"WASM module initialized!".into());

    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("should have a document on window");

    create_hello_world(&document)?;

    Ok(())
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

fn create_hello_world(document: &Document) -> Result<(), JsValue> {
    let body = document.body().expect("document should have a body");

    let container = document.create_element("div")?;
    container.set_id("wasm-container");

    let heading = document.create_element("h1")?;
    heading.set_text_content(Some("Hello from Rust + WebAssembly!"));

    let paragraph = document.create_element("p")?;
    paragraph.set_text_content(Some(
        "This page is rendered using Rust compiled to WebAssembly.",
    ));

    let button = document.create_element("button")?;
    button.set_text_content(Some("Click me!"));

    let closure = Closure::wrap(Box::new(move || {
        console::log_1(&"Button clicked from Rust!".into());
        web_sys::window().and_then(|w| w.alert_with_message("Hello from Rust + WASM!").ok());
    }) as Box<dyn Fn()>);

    button
        .dyn_ref::<HtmlElement>()
        .expect("button should be an HtmlElement")
        .set_onclick(Some(closure.as_ref().unchecked_ref()));

    closure.forget();

    container.append_child(&heading)?;
    container.append_child(&paragraph)?;
    container.append_child(&button)?;
    body.append_child(&container)?;

    Ok(())
}
