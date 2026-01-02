# front-ltx

WebAssembly frontend for the llms.txt project, built with Rust.

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
# From the src/front_ltx directory
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
# From the src/front_ltx/www directory
cd www
python3 -m http.server 8080
```

Then open your browser to: http://localhost:8080

### Option 2: Using miniserve (Recommended)

```bash
# From the src/front_ltx/www directory
cd www
miniserve . --port 8080
```

Then open your browser to: http://localhost:8080

### Option 3: Using Node.js http-server

```bash
# From the src/front_ltx/www directory
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
src/front_ltx/
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

## Next Steps

To extend this frontend:

1. Add more Rust functions and export them with `#[wasm_bindgen]`
2. Create more complex UI components
3. Integrate with the API backend (api_ltx)
4. Add a bundler (webpack, vite, etc.) for more sophisticated builds
5. Consider using a framework like Yew, Leptos, or Dioxus for larger applications
