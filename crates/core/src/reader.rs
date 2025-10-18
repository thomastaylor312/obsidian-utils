use clap::Args;
use std::fs::Metadata;
use std::path::{Path, PathBuf};

use anyhow::Result;

#[derive(Args, Debug)]
pub struct ReaderOpts {
    /// Whether to recurse into subdirectories when reading the vault. Defaults to true
    #[arg(long, default_value_t = true)]
    pub recurse: bool,

    /// The directory containing the vault to operate on
    // TODO: Make this optional once we support a list of files from stdin
    pub vault_dir: PathBuf,
}

impl ReaderOpts {
    /// Read the directory specified in the options, returning a list of all files found. This is a
    /// convenience method around [read_dir].
    pub fn read_dir(&self) -> Result<Vec<FileEntry>> {
        read_dir(&self.vault_dir, self.recurse)
    }
}

pub struct FileEntry {
    pub path: PathBuf,
    pub metadata: Metadata,
}

// TODO: Figure out if we can turn this into an iter instead so we don't have to allocate a big Vec
// of all entries before processing them

/// Read a directory from disk, returning a list of all files found. If recurse is true, this will
/// recurse into subdirectories as well.
pub fn read_dir(path: impl AsRef<Path>, recurse: bool) -> Result<Vec<FileEntry>> {
    let mut entries = vec![];
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() && recurse {
            entries.extend(read_dir(&p, true)?);
        } else if metadata.is_file() {
            entries.push(FileEntry { path: p, metadata });
        }
    }
    Ok(entries)
}
