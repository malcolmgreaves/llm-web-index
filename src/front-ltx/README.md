# front-ltx

WebAssembly frontend for the llms.txt generation system. Provides a web-based user interface for submitting URLs, monitoring job status, and viewing generated llms.txt files. Built entirely in Rust and compiled to WebAssembly.

## Overview

The `front-ltx` crate provides:

- **WASM-based UI**: Browser-native interface compiled from Rust
- **API integration**: Communicates with the API server for all operations
- **Authentication flow**: Handles login/logout when authentication is enabled
- **Job submission**: Form for creating new llms.txt generation jobs
- **Status monitoring**: Real-time job status display
- **Result viewing**: Display and download generated llms.txt files
- **Responsive design**: Works on desktop and mobile browsers

The frontend is served as static files by the API server and runs entirely in the browser.

## Prerequisites

Before you can build and run this WebAssembly frontend, you need to install `wasm-pack`:

```bash
cargo install wasm-pack
```

For serving the application locally, you'll need a simple HTTP server. Install one of these:

```bash
# Option 1: Python (usually pre-installed)
python3 --version

# Option 2: Node.js http-server
npm install -g http-server

# Option 3: Rust miniserve (recommended)
cargo install miniserve
```

## Building

Build the WebAssembly module using `wasm-pack`:

```bash
# From the src/front-ltx directory
wasm-pack build --target web --out-dir www/pkg
```

This will:
- Compile the Rust code to WebAssembly
- Generate JavaScript bindings
- Place the output in `www/pkg/` directory

### Build Options

For a production-optimized build:

```bash
wasm-pack build --target web --out-dir www/pkg --release
```

For development with debug symbols:

```bash
wasm-pack build --target web --out-dir www/pkg --dev
```

## Running in a Web Browser

After building, you need to serve the files over HTTP (not `file://`) because of CORS restrictions with ES modules.

### Option 1: Using Python (Simple)

```bash
# From the src/front-ltx/www directory
cd www
python3 -m http.server 8080
```

Then open your browser to: http://localhost:8080

### Option 2: Using miniserve (Recommended)

```bash
# From the src/front-ltx/www directory
cd www
miniserve . --port 8080
```

Then open your browser to: http://localhost:8080

### Option 3: Using Node.js http-server

```bash
# From the src/front-ltx/www directory
cd www
http-server -p 8080
```

Then open your browser to: http://localhost:8080

## Development Workflow

For iterative development:

```bash
# 1. Make changes to src/lib.rs
# 2. Rebuild the WASM module
wasm-pack build --target web --out-dir www/pkg

# 3. Refresh your browser (the server should still be running)
```

### Watch Mode (Optional)

For automatic rebuilds on file changes, you can use `cargo-watch`:

```bash
cargo install cargo-watch

# Run this in one terminal
cargo watch -i www/ -s "wasm-pack build --target web --out-dir www/pkg"

# Run your server in another terminal
cd www && miniserve . --port 8080
```

## Project Structure

```
src/front-ltx/
├── Cargo.toml          # Rust package configuration
├── README.md           # This file
├── src/
│   └── lib.rs          # Main Rust source code
└── www/
    ├── index.html      # HTML entry point
    └── pkg/            # Generated WASM output (gitignored)
        ├── front_ltx.js
        ├── front_ltx_bg.wasm
        └── ...
```

## Features

This minimal example demonstrates:
- WebAssembly module initialization
- DOM manipulation from Rust
- Event handling (button clicks)
- Calling JavaScript APIs from Rust (console.log, alert)
- Modern ES6 module imports

## Browser Compatibility

This setup requires a modern browser with support for:
- WebAssembly
- ES6 modules
- JavaScript `async`/`await`

Supported browsers:
- Chrome/Edge 61+
- Firefox 60+
- Safari 11+
- Opera 48+

## Troubleshooting

### "Failed to load WASM" error

1. Ensure you've built the project with `wasm-pack build`
2. Check that you're serving over HTTP, not opening the HTML file directly
3. Check browser console for specific error messages
4. Verify that `www/pkg/` directory exists and contains the WASM files

### CORS errors

Make sure you're using an HTTP server to serve the files, not opening `index.html` directly with `file://` protocol.

### Module not found errors

Ensure the import path in `index.html` matches the output directory from `wasm-pack build`.

## Integration with the API Server

The frontend is automatically served by the API server when you run:

```bash
# Via just command
just serve

# Via Docker Compose
docker compose up

# Via cargo directly
cargo run -p api-ltx
```

The API server serves:
- `GET /` - Returns `www/index.html`
- `GET /pkg/*` - Serves WASM and JS files from `www/pkg/`

When authentication is enabled (`ENABLE_AUTH=1`), the frontend automatically shows the login page first.

## Development Workflow

The recommended development workflow is:

1. **Backend Development**:
   ```bash
   # Terminal 1: Run the full stack
   docker compose up
   ```

2. **Frontend Development**:
   ```bash
   # Terminal 2: Watch for frontend changes and rebuild
   cd src/front-ltx
   cargo watch -i www/ -s "cargo build --target wasm32-unknown-unknown --release && \
     wasm-bindgen ../../target/wasm32-unknown-unknown/release/front_ltx.wasm \
       --out-dir www/pkg --target web"
   ```

3. **Access the application**: Open https://localhost:3000 in your browser

Changes to frontend Rust code will automatically rebuild the WASM, and the browser will reload.

## Building for Production

Production builds optimize the WASM for size:

```bash
# From project root
just release
```

This:
1. Builds the WASM frontend in release mode
2. Runs `wasm-opt` to optimize the WASM binary
3. Builds all backend services in release mode

The optimized WASM can be 50-80% smaller than the unoptimized version.

## Architecture

The frontend uses:

- **wasm-bindgen**: JavaScript ↔ Rust interop
- **web-sys**: Web API bindings (DOM, fetch, etc.)
- **js-sys**: JavaScript standard library bindings
- **serde**: JSON serialization for API communication

### API Communication

All API requests use the browser's `fetch` API via `web-sys`:

```rust
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

// Example: Create a job
let url = "/api/jobs";
let body = json!({"url": "https://example.com"});

let request = Request::new_with_str_and_init(
    url,
    RequestInit::new().method("POST").body(Some(&body))
)?;

let response = JsFuture::from(window.fetch_with_request(&request)).await?;
```

### Session Management

When authentication is enabled:
- Session cookies are automatically handled by the browser
- The frontend checks authentication status on load
- Failed authentication redirects to the login page
- Logout clears the session cookie

## Testing

Frontend testing options:

```bash
# Unit tests (runs in Node.js/Deno environment)
cargo test -p front-ltx

# Browser-based testing (requires wasm-pack test)
wasm-pack test --headless --firefox
wasm-pack test --headless --chrome
```

## Browser Compatibility

Requires modern browser features:
- **WebAssembly**: All modern browsers (2017+)
- **ES6 Modules**: Dynamic imports
- **Async/Await**: Promise handling
- **Fetch API**: HTTP requests

Supported browsers:
- Chrome/Edge 61+
- Firefox 60+
- Safari 11+
- Opera 48+

## Deployment Considerations

### Content Security Policy

When deploying, consider adding CSP headers:

```
Content-Security-Policy: default-src 'self';
  script-src 'self' 'wasm-unsafe-eval';
  style-src 'self' 'unsafe-inline';
```

The `'wasm-unsafe-eval'` is required for WebAssembly.

### HTTPS Requirement

Modern browsers require HTTPS for many web APIs. The API server uses HTTPS by default.

### Caching

Configure appropriate cache headers for:
- WASM files: Long cache (content-hashed filenames recommended)
- JS bindings: Long cache (content-hashed filenames recommended)
- index.html: Short cache or no-cache

## Advanced Development

### Using a UI Framework

For more complex UIs, consider integrating a Rust web framework:

- **Yew**: React-like framework for Rust + WASM
- **Leptos**: Full-stack Rust framework
- **Dioxus**: Cross-platform UI framework

Example with Yew:

```bash
cargo add yew --features csr
```

### Adding Webpack/Vite

For advanced bundling, asset processing, and hot reload:

```bash
# Initialize with Vite
npm init vite@latest

# Configure to use wasm-pack
npm install vite-plugin-wasm
```

### Debugging WASM

Enable source maps for better debugging:

```bash
wasm-pack build --target web --out-dir www/pkg --dev
```

Then use browser DevTools to debug Rust code directly.

## Related Documentation

- [wasm-bindgen Book](https://rustwasm.github.io/wasm-bindgen/) - Core WASM interop
- [web-sys Documentation](https://rustwasm.github.io/wasm-bindgen/api/web_sys/) - Web API bindings
- [Project Root README](../../README.md) - Overall project documentation
- [api-ltx README](../api-ltx/README.md) - API server that serves this frontend
- [data-model-ltx README](../data-model-ltx/README.md) - Data models used in API communication
