//! Configuration options for llms.txt generation.

/// Configuration options for the generator.
#[derive(Debug, Clone)]
pub struct GeneratorOptions {
    /// Glob patterns for paths to exclude from processing
    pub exclude_paths: Vec<String>,
    /// Glob patterns for paths to include (if specified, only these paths are processed)
    pub include_paths: Vec<String>,
    /// Substitution commands to apply to page titles (sed-style: s/pattern/replacement/flags)
    pub replace_title: Vec<String>,
    /// Maximum number of concurrent requests (default: 5)
    pub concurrency: usize,
    /// Optional title to use for the generated document
    pub title: Option<String>,
    /// Optional description to use for the generated document
    pub description: Option<String>,
}

impl Default for GeneratorOptions {
    fn default() -> Self {
        Self {
            exclude_paths: Vec::new(),
            include_paths: Vec::new(),
            replace_title: Vec::new(),
            concurrency: 5,
            title: None,
            description: None,
        }
    }
}

impl GeneratorOptions {
    /// Creates a new builder for GeneratorOptions.
    pub fn builder() -> GeneratorOptionsBuilder {
        GeneratorOptionsBuilder::default()
    }
}

/// Builder for GeneratorOptions.
#[derive(Debug, Clone, Default)]
pub struct GeneratorOptionsBuilder {
    exclude_paths: Vec<String>,
    include_paths: Vec<String>,
    replace_title: Vec<String>,
    concurrency: Option<usize>,
    title: Option<String>,
    description: Option<String>,
}

impl GeneratorOptionsBuilder {
    /// Adds a path pattern to exclude.
    pub fn exclude_path(mut self, pattern: String) -> Self {
        self.exclude_paths.push(pattern);
        self
    }

    /// Adds multiple path patterns to exclude.
    pub fn exclude_paths(mut self, patterns: Vec<String>) -> Self {
        self.exclude_paths.extend(patterns);
        self
    }

    /// Adds a path pattern to include.
    pub fn include_path(mut self, pattern: String) -> Self {
        self.include_paths.push(pattern);
        self
    }

    /// Adds multiple path patterns to include.
    pub fn include_paths(mut self, patterns: Vec<String>) -> Self {
        self.include_paths.extend(patterns);
        self
    }

    /// Adds a title replacement command (sed-style: s/pattern/replacement/flags).
    pub fn replace_title(mut self, command: String) -> Self {
        self.replace_title.push(command);
        self
    }

    /// Adds multiple title replacement commands.
    pub fn replace_titles(mut self, commands: Vec<String>) -> Self {
        self.replace_title.extend(commands);
        self
    }

    /// Sets the concurrency level (number of simultaneous requests).
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = Some(concurrency);
        self
    }

    /// Sets the document title.
    pub fn title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    /// Sets the document description.
    pub fn description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Builds the GeneratorOptions.
    pub fn build(self) -> GeneratorOptions {
        GeneratorOptions {
            exclude_paths: self.exclude_paths,
            include_paths: self.include_paths,
            replace_title: self.replace_title,
            concurrency: self.concurrency.unwrap_or(5),
            title: self.title,
            description: self.description,
        }
    }
}
