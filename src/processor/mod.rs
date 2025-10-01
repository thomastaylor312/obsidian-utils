mod file_handler;
pub use file_handler::FileHandler;

pub struct ParsedFile {
    // TODO: Create some sort of dynamic structure for the content that can pull a generate type
    // schema from the front matter
    pub front_matter: Option<serde_norway::Value>,
    // TODO: Add more fields here as needed
}

/// A trait for processing pre-parsed markdown files
#[async_trait::async_trait]
pub trait Processor {
    async fn process(&self, file: &ParsedFile) -> anyhow::Result<()>;
}
