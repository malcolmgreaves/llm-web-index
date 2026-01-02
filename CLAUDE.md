# CLAUDE.md

This file helps Claude Code and other AI assistants understand the anchor-stream project structure, conventions, and workflows.

## Code Quality Standards

**Rust:**
- Formatting: `cargo fmt`
- Linting: `cargo clippy --fix`
- Unused dependencies: `cargo machete`
- Type checking: `cargo check --all-targets`

## Testing

**Rust:**
```bash
# Run tests with coverage (generates HTML report in target/llvm-cov/html/)
cargo llvm-cov --all-targets --workspace --html

# Run benchmarks
cargo bench --all-targets --workspace
```

## Conventions

### Naming
- **Rust**: snake_case for functions/variables, PascalCase for types
- **Git commits**: Descriptive commit messages (see git log for style)

### Dependencies
- **Rust workspace**: Consider adding to `[workspace.dependencies]` if shared
- **Rust local**: Add directly to crate's `Cargo.toml`

## Coding Style & Principles

### General Philosophy

**Use functional programming style as the default.** Prefer immutability everywhere unless there's a compelling reason not to.

**Key Principles:**
- **Immutability first**: Make everything immutable by default
- **Encapsulate mutability**: When mutability is needed, encapsulate it behind functional interfaces
- **Natural mutability is OK**: Don't force naturally mutable abstractions into immutable ones
- **Performance matters**: Consider mutability to save memory, but avoid unnecessary copies
- **Keep it simple**: Prefer simpler solutions and direct code structures
- **Avoid OOP by default**: Only use object-oriented patterns when the problem truly models objects
- **Prefer composition**: Use struct/dataclass + functions instead of class hierarchies
- **Direct over abstract**: Some repetition is better than overly complex or strict abstractions
- **Abstract purposefully**: Only create abstractions for code reuse or when fundamental to the concept

### Rust-Specific Guidelines

**Error Handling:**
- Make descriptive `enum Error` types to encode custom error logic for each crate/module
- **Always use** `Result` whenever something could go wrong
- **Always use** `?` operator over `.unwrap()`
- `.unwrap()` is acceptable in **unit tests only!!!** (_because we'd be checking for failure condition(s) immediately!_)

**Lifetimes & Ownership:**
- Use lifetimes effectively to express borrowing relationships
- If lifetime management becomes too complex with `async` code, use `Arc` instead
- Prefer borrowing over cloning when possible

**Traits:**
- Only create a `trait` if there are multiple implementations
- Otherwise, prefer functions and structs with `impl` blocks

**Example patterns:**
```rust
// Good: Result with ? operator
fn process_data(path: &Path) -> Result<Data, Error> {
    let content = fs::read_to_string(path)?;
    parse_content(&content)
}

// Good: Descriptive error enum
enum ParseError {
    InvalidFormat { line: usize, reason: String },
    MissingField(String),
    IoError(io::Error),
}

// Prefer: Functions over single-implementation traits
impl DataProcessor {
    fn process(&self, data: &[u8]) -> Result<Output, Error> { ... }
}

// Instead of:
trait Processor {
    fn process(&self, data: &[u8]) -> Result<Output, Error>;
}
```

### When to Break These Rules

- **Natural mutability**: Use mutable patterns when they're the clearest expression (e.g., building up state, caches, iterative algorithms)
- **Performance critical**: Profile first, then optimize with mutability if needed
- **Framework requirements**: Some libraries require OOP patterns - use them when necessary
- **Domain modeling**: If the domain is naturally object-oriented, model it that way

