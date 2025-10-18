use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use serde::{
    Deserialize, Serialize,
    de::{MapAccess, Visitor},
};

pub mod parser;

/// Information about links associated with a file
#[derive(Debug, Serialize, Deserialize)]
pub struct FileLinks {
    /// Whether or not the file actually exists in the vault. If this is false, by definition it will
    /// only have backlinks
    pub exists: bool,
    /// All links found in the file to other vault files. Does not include external links
    pub links: HashSet<PathBuf>,
    /// All backlinks found in other files pointing to this file
    #[serde(skip)]
    pub backlinks: HashSet<PathBuf>,
}

#[derive(Debug, Default, Serialize)]
pub struct Links(HashMap<PathBuf, FileLinks>);

impl Links {
    /// Create a new, empty Links struct
    pub fn new() -> Self {
        Self::default()
    }

    fn with_capacity(cap: usize) -> Self {
        Links(HashMap::with_capacity(cap))
    }

    /// Insert a link from one file to another. The `from` value should always be a path to a file
    /// that exists.
    ///
    /// NOTE: All paths added to `Links` _must_ be the same path value. So if you have canonicalized
    /// one path, _all_ other paths must be canonicalized the same way or this will end up with
    /// duplicate paths. This is specifically so the caller can determine which kind of path styles
    /// they want to use when calling the function (relative or fully qualified).
    pub fn insert_link(&mut self, from: PathBuf, to: PathBuf) {
        // We aren't using the entry API here because we want to avoid allocating the PathBuf for
        // `from` unless we need to
        if let Some(file_links) = self.0.get_mut(&from) {
            file_links.exists = true;
            file_links.links.insert(to.clone());
        } else {
            self.0.insert(
                from.clone(),
                FileLinks {
                    exists: true,
                    links: HashSet::from([to.clone()]),
                    backlinks: HashSet::new(),
                },
            );
        }
        // Now insert the backlink. Right now we always clone `to`, but if we really want to squeeze
        // out less allocations we could potentially use a more sophisticated approach to avoid
        // cloning.
        if let Some(file_links) = self.0.get_mut(&to) {
            file_links.backlinks.insert(from);
        } else {
            self.0.insert(
                to,
                FileLinks {
                    exists: false,
                    links: HashSet::new(),
                    backlinks: HashSet::from([from]),
                },
            );
        }
    }

    /// A convenience wrapper that "bulk adds" all links from one file to multiple others. This is
    /// effectively just a loop around `insert_link`.
    pub fn insert_links<I>(&mut self, from: PathBuf, to_files: I)
    where
        I: IntoIterator<Item = PathBuf>,
    {
        for to in to_files {
            self.insert_link(from.clone(), to);
        }
    }

    /// Add a file that exists but has no outgoing links. This allows adding nodes into the graph
    /// that are orphans, or while manually constructing links.
    pub fn insert_file(&mut self, path: PathBuf) {
        self.0.entry(path).or_insert(FileLinks {
            exists: true,
            links: HashSet::new(),
            backlinks: HashSet::new(),
        });
    }

    /// Get the link info for a single file, if it exists
    pub fn get<Q: Borrow<PathBuf>>(&self, path: Q) -> Option<&FileLinks> {
        self.0.get(path.borrow())
    }

    /// Get an iterator over all files and their associated link info
    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &FileLinks)> {
        self.0.iter()
    }

    /// Traverse all links in the graph depth-first starting from the given file path, returning an
    /// iterator of all visited file paths. The first item will always be the starting file. Cycles
    /// are avoided.
    ///
    /// Currently this function does not guarantee any specific order of traversal beyond
    /// depth-first. Each time this function is called, the iteration order may differ.
    // NOTE(thomastaylor312): If we want to have a consistent iteration order, we could use a
    // BTreeMap as the underlying structure instead of a HashMap.
    pub fn traverse_links_dfs<'a>(
        &'a self,
        start: &'a Path,
    ) -> impl Iterator<Item = (&'a Path, &'a FileLinks)> + 'a {
        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        if let Some(start_links) = self.0.get(start) {
            stack.push((start, start_links));
        }

        std::iter::from_fn(move || {
            while let Some((current_path, current)) = stack.pop() {
                if visited.insert(current_path) {
                    for link in &current.links {
                        if let Some(linked_file) = self.0.get(link) {
                            stack.push((link.as_path(), linked_file));
                        }
                    }
                    return Some((current_path, current));
                }
            }
            None
        })
    }

    /// Traverse all backlinks in the graph depth-first starting from the given file path, returning an
    /// iterator of all visited file paths. The first item will always be the starting file. Cycles
    /// are avoided.
    ///
    /// Currently this function does not guarantee any specific order of traversal beyond
    /// depth-first. Each time this function is called, the iteration order may differ.
    pub fn traverse_backlinks_dfs<'a>(
        &'a self,
        start: &'a Path,
    ) -> impl Iterator<Item = (&'a Path, &'a FileLinks)> + 'a {
        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        if let Some(start_links) = self.0.get(start) {
            stack.push((start, start_links));
        }

        std::iter::from_fn(move || {
            while let Some((current_path, current)) = stack.pop() {
                if visited.insert(current_path) {
                    for backlink in &current.backlinks {
                        if let Some(linked_file) = self.0.get(backlink) {
                            stack.push((backlink.as_path(), linked_file));
                        }
                    }
                    return Some((current_path, current));
                }
            }
            None
        })
    }
}

impl IntoIterator for Links {
    type Item = (PathBuf, FileLinks);
    type IntoIter = std::collections::hash_map::IntoIter<PathBuf, FileLinks>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// We use a custom deserialization for Links so that we don't have to walk the whole graph twice. We
// can reassemble backlinks on deserialization. We're increasing space complexity here because we
// essentially allocate each FileLinks and path buf twice (once from it deserializing and once while
// inserting) but gives us more speed on bigger graphs because we only iterate once while
// deserializing the data. Maybe this optimization doesn't matter, in which case we can just do a
// deserialize method that deserializes the HashMap and then iterates over it to reconstruct
// backlinks

struct LinksVisitor;

impl<'de> Visitor<'de> for LinksVisitor {
    // The type that our Visitor is going to produce.
    type Value = Links;

    // Format a message stating what data this Visitor expects to receive.
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("HashMap<PathBuf, FileLinks>")
    }

    // Deserialize MyMap from an abstract "map" provided by the
    // Deserializer. The MapAccess input is a callback provided by
    // the Deserializer to let us see each entry in the map.
    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut links = access
            .size_hint()
            .map(Links::with_capacity)
            .unwrap_or_default();

        // While there are entries remaining in the input, add them
        // into our map.
        while let Some((key, value)) = access.next_entry::<PathBuf, FileLinks>()? {
            links.insert_links(key, value.links);
        }

        Ok(links)
    }
}

impl<'de> Deserialize<'de> for Links {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(LinksVisitor)
    }
}
