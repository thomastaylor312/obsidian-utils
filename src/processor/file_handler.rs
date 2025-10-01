pub struct FileHandler {
    processors: Vec<Box<dyn super::Processor + Send + Sync>>,
}

impl FileHandler {
    pub fn new(processors: Vec<Box<dyn super::Processor + Send + Sync>>) -> Self {
        Self { processors }
    }
}

impl notify::EventHandler for FileHandler {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        // TODO: filter out events (anything that isn't .md, a create/delete/update event, not a file)
        // TODO: Parse file to AST and then parse front matter
        todo!("Implement file handling logic here");
    }
}
