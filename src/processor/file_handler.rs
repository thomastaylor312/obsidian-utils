use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use comrak::{Arena, Options};
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind};
use tracing::error;

pub struct FileHandler {
    processors: Vec<Box<dyn super::Processor + Send + Sync>>,
    base_dir: PathBuf,
}

impl FileHandler {
    pub async fn new(
        base_dir: impl AsRef<Path>,
        processors: Vec<Box<dyn super::Processor + Send + Sync>>,
    ) -> anyhow::Result<Self> {
        let metadata = tokio::fs::metadata(&base_dir).await?;
        if !metadata.is_dir() {
            anyhow::bail!("Path {} is not a directory", base_dir.as_ref().display())
        }
        let this = Self {
            processors,
            base_dir: base_dir.as_ref().to_path_buf(),
        };
        this.process_dir().await?;
        Ok(this)
    }

    pub async fn handle_event(&self, event: notify::Result<Event>) {
        let event = match event {
            Ok(evt) => evt,
            Err(err) => {
                error!(%err, "error from fsnotify");
                return;
            }
        };
        if event.need_rescan() {
            if let Err(e) = Box::pin(self.process_dir()).await {
                error!(err = %e, "Unable to resync dir")
            }
            return;
        }
        let maybe_path = match event.kind {
            EventKind::Any | EventKind::Create(_) => event.paths.into_iter().next(),
            EventKind::Modify(ModifyKind::Name(RenameMode::To))
            | EventKind::Modify(ModifyKind::Data(_))
            | EventKind::Modify(ModifyKind::Any)
            | EventKind::Modify(ModifyKind::Other) => event.paths.into_iter().next(),
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                // TODO: Handle delete of path
                event.paths.into_iter().nth(1)
            }
            EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                let p = match event.paths.into_iter().next() {
                    Some(p) => p,
                    None => return,
                };
                // TODO: Same as delete
                return;
            }

            _ => {
                tracing::debug!(path = ?event.paths, kind = ?event.kind, "Ignoring event");
                return;
            }
        };
        let path = match maybe_path {
            Some(p) => p,
            None => return,
        };
        let metadata = match tokio::fs::metadata(&path).await {
            Ok(metadata) => metadata,
            Err(e) => {
                error!(err = %e, ?path, "Error when reading metadata for file");
                return;
            }
        };
        if !metadata.is_file() {
            return;
        }
        if !path
            .extension()
            .and_then(OsStr::to_str)
            .map(|ext| ext.eq_ignore_ascii_case("md"))
            .unwrap_or(false)
        {
            return;
        }

        // TODO: Parse file to AST and then parse front matter
        let arena = Arena::new();
        let content = match tokio::fs::read_to_string(&path).await {
            Ok(s) => s,
            Err(e) => {
                error!(err = %e, path = %path.display(), "Failed to read file from path");
                return;
            }
        };
        // TODO: Configure options
        let opts = Options::default();
        let ast = comrak::parse_document(&arena, &content, &opts);
        todo!("Implement file handling logic here");
    }

    async fn process_dir(&self) -> anyhow::Result<()> {
        let mut stack: Vec<PathBuf> = vec![self.base_dir.clone()];

        // TODO: Maybe run some of these in parallel to speed things up

        while let Some(dir) = stack.pop() {
            let mut rd = match tokio::fs::read_dir(&dir).await {
                Ok(rd) => rd,
                Err(e) => {
                    error!(%e, path = %dir.display(), "failed to read directory");
                    continue;
                }
            };

            while let Some(entry) = rd.next_entry().await? {
                let path = entry.path();
                match entry.metadata().await {
                    Ok(md) => {
                        if md.is_dir() {
                            stack.push(path);
                        } else if md.is_file()
                            && path
                                .extension()
                                .and_then(OsStr::to_str)
                                .map(|ext| ext.eq_ignore_ascii_case("md"))
                                .unwrap_or(false)
                        {
                            let evt = Event::new(notify::EventKind::Modify(
                                notify::event::ModifyKind::Data(notify::event::DataChange::Content),
                            ));
                            self.handle_event(Ok(evt.add_path(path))).await;
                        }
                    }
                    Err(e) => {
                        error!(%e, path = %path.display(), "failed to read metadata for entry");
                    }
                }
            }
        }

        Ok(())
    }
}
