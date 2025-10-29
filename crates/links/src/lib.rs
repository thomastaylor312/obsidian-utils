use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

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
    pub backlinks: HashSet<PathBuf>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Links(HashMap<PathBuf, FileLinks>);

impl Links {
    /// Create a new, empty Links struct
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new Links struct with the given capacity
    pub fn with_capacity(cap: usize) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::iter::FromIterator;

    #[test]
    fn insert_link_records_forward_and_back_references() {
        let mut links = Links::new();
        let from = PathBuf::from("/vault/source.md");
        let to = PathBuf::from("/vault/target.md");

        links.insert_link(from.clone(), to.clone());

        let from_entry = links.get(&from).expect("from entry missing");
        assert!(from_entry.exists);
        assert!(from_entry.links.contains(&to));
        assert!(from_entry.backlinks.is_empty());

        let to_entry = links.get(&to).expect("to entry missing");
        assert!(!to_entry.exists);
        assert!(to_entry.links.is_empty());
        assert!(to_entry.backlinks.contains(&from));
    }

    #[test]
    fn insert_links_bulk_adds_all_targets() {
        let mut links = Links::new();
        let from = PathBuf::from("/vault/origin.md");
        let targets = vec![
            PathBuf::from("/vault/a.md"),
            PathBuf::from("/vault/b.md"),
            PathBuf::from("/vault/c.md"),
        ];

        links.insert_links(from.clone(), targets.clone());

        let from_entry = links.get(&from).expect("origin entry missing");
        assert_eq!(from_entry.links.len(), targets.len());
        for target in &targets {
            assert!(from_entry.links.contains(target));
            let target_entry = links.get(target).expect("target entry missing");
            assert!(target_entry.backlinks.contains(&from));
        }
    }

    #[test]
    fn insert_file_creates_orphan_entry() {
        let mut links = Links::new();
        let orphan = PathBuf::from("/vault/orphan.md");

        links.insert_file(orphan.clone());

        let entry = links.get(&orphan).expect("orphan entry missing");
        assert!(entry.exists);
        assert!(entry.links.is_empty());
        assert!(entry.backlinks.is_empty());
    }

    #[test]
    fn traverse_links_depth_first_visits_all_nodes() {
        let mut links = Links::new();
        let root = PathBuf::from("/vault/root.md");
        let child_a = PathBuf::from("/vault/a.md");
        let child_b = PathBuf::from("/vault/b.md");
        let grandchild = PathBuf::from("/vault/c.md");

        links.insert_link(root.clone(), child_a.clone());
        links.insert_link(root.clone(), child_b.clone());
        links.insert_link(child_a.clone(), grandchild.clone());

        let order: Vec<PathBuf> = links
            .traverse_links_dfs(root.as_path())
            .map(|(path, _)| path.to_path_buf())
            .collect();

        assert_eq!(order.len(), 4);
        assert_eq!(order.first(), Some(&root));

        let index_of = |value: &PathBuf| order.iter().position(|p| p == value).unwrap();
        let idx_child_a = index_of(&child_a);
        let idx_grandchild = index_of(&grandchild);

        assert!(
            idx_grandchild > idx_child_a,
            "grandchild should appear after its parent when traversing depth-first: {:?}",
            order
        );

        let between = &order[idx_child_a + 1..idx_grandchild];
        assert!(
            between.iter().all(|entry| entry == &grandchild),
            "nodes between child_a and its descendant should belong to that subtree (DFS property), observed: {:?}",
            between
        );

        let visited: HashSet<PathBuf> = HashSet::from_iter(order);
        let expected = HashSet::from_iter([
            root.clone(),
            child_a.clone(),
            child_b.clone(),
            grandchild.clone(),
        ]);
        assert_eq!(visited, expected);
    }

    #[test]
    fn traverse_backlinks_depth_first_visits_all_nodes() {
        let mut links = Links::new();
        let root = PathBuf::from("/vault/root.md");
        let child_a = PathBuf::from("/vault/a.md");
        let child_b = PathBuf::from("/vault/b.md");
        let grandchild = PathBuf::from("/vault/c.md");

        links.insert_link(child_a.clone(), root.clone());
        links.insert_link(child_b.clone(), root.clone());
        links.insert_link(grandchild.clone(), child_b.clone());

        let order: Vec<PathBuf> = links
            .traverse_backlinks_dfs(root.as_path())
            .map(|(path, _)| path.to_path_buf())
            .collect();

        assert_eq!(order.len(), 4);
        assert_eq!(order.first(), Some(&root));

        let index_of = |value: &PathBuf| order.iter().position(|p| p == value).unwrap();
        let idx_child_b = index_of(&child_b);
        let idx_grandchild = index_of(&grandchild);

        assert!(
            idx_grandchild > idx_child_b,
            "backlink traversal should visit a node before its descendants: {:?}",
            order
        );

        let between = &order[idx_child_b + 1..idx_grandchild];
        assert!(
            between.iter().all(|entry| entry == &grandchild),
            "nodes between child_b and its descendant backlink should belong to that subtree, observed: {:?}",
            between
        );

        let visited: HashSet<PathBuf> = HashSet::from_iter(order);
        let expected = HashSet::from_iter([
            root.clone(),
            child_a.clone(),
            child_b.clone(),
            grandchild.clone(),
        ]);
        assert_eq!(visited, expected);
    }
}
