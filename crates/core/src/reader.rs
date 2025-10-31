use clap::Args;
use std::fs::Metadata;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::Result;

#[derive(Args, Debug)]
pub struct ReaderOpts {
    /// Whether to recurse into subdirectories when reading the vault. Defaults to true. This also
    /// applies when passing directories via stdin.
    #[arg(long, default_value_t = true)]
    pub recurse: bool,

    /// A directory containing files to read. If this is passed, any files passed from stdin will be
    /// ignored.
    ///
    /// When reading from stdin, if --recurse is set to true, files in directories will also be
    /// read. Otherwise, only files will be read and all other paths ignored.
    pub dir: Option<PathBuf>,
}

impl ReaderOpts {
    /// Get this list of file entries from stdin or by the directory specified in the options.
    pub fn read_files(&self) -> Result<Vec<FileEntry>> {
        // If a directory is explicitly provided, use it regardless of stdin state
        if let Some(dir) = &self.dir {
            read_dir(dir, self.recurse)
        } else if !std::io::stdin().is_terminal() {
            // Only read from stdin if no directory was provided
            read_stdin(self.recurse)
        } else {
            Err(anyhow::anyhow!(
                "No vault directory specified and no input from stdin. Cannot proceed."
            ))
        }
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

pub fn read_stdin(recurse: bool) -> Result<Vec<FileEntry>> {
    let mut entries = vec![];
    for line in std::io::stdin().lines() {
        let line = line?;
        let path = PathBuf::from(line.trim());
        let metadata = std::fs::metadata(&path)?;
        if metadata.is_dir() && recurse {
            entries.extend(read_dir(&path, true)?);
            continue;
        } else if !metadata.is_file() {
            continue;
        }
        entries.push(FileEntry { path, metadata });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-vault")
    }

    #[test]
    fn read_dir_non_recursive_ignores_subdirectories() -> anyhow::Result<()> {
        let vault = vault_path();
        let entries = read_dir(&vault, false)?;
        let mut relative_paths: Vec<PathBuf> = entries
            .iter()
            .map(|entry| entry.path.strip_prefix(&vault).unwrap().to_path_buf())
            .collect();
        relative_paths.sort();

        assert!(
            relative_paths.contains(&PathBuf::from("Test.md")),
            "Expected root markdown file to be discovered"
        );
        assert!(
            !relative_paths.contains(&PathBuf::from("other/Other.md")),
            "Subdirectory files should not be returned when recurse is false"
        );
        assert!(
            relative_paths.contains(&PathBuf::from("notes.txt")),
            "Non-markdown files are still surfaced by the directory reader"
        );

        Ok(())
    }

    #[test]
    fn read_dir_recursive_finds_nested_files_only() -> anyhow::Result<()> {
        let vault = vault_path();
        let entries = read_dir(&vault, true)?;
        let mut relative_paths: Vec<PathBuf> = entries
            .iter()
            .map(|entry| entry.path.strip_prefix(&vault).unwrap().to_path_buf())
            .collect();
        relative_paths.sort();

        assert!(
            relative_paths.contains(&PathBuf::from("other/Other.md")),
            "Expected nested markdown file when recurse is true"
        );
        assert!(
            !relative_paths.iter().any(|p| p.ends_with("other")),
            "Directories should not be returned"
        );
        assert!(
            entries.iter().all(|entry| entry.metadata.is_file()),
            "Directory reader should only produce file metadata"
        );

        Ok(())
    }
}
