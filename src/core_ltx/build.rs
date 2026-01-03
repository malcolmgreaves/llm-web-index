fn main() {
    // Declare the custom cfg so rustc knows about it
    println!("cargo::rustc-check-cfg=cfg(has_openai_key)");

    // Emit a custom cfg flag if OPENAI_API_KEY is set
    if let Ok(openai_api_key) = std::env::var("OPENAI_API_KEY")
        && !openai_api_key.is_empty()
    {
        println!("cargo:rustc-cfg=has_openai_key");
    }
}
