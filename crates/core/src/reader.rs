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
