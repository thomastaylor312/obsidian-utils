use std::{
    borrow::Borrow,
    collections::{BTreeMap, BTreeSet},
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
    pub links: BTreeSet<PathBuf>,
    /// All backlinks found in other files pointing to this file
    pub backlinks: BTreeSet<PathBuf>,
}

impl FileLinks {
    /// Returns true if the file is an orphan (i.e. it has no links and no backlinks)
    pub fn is_orphan(&self) -> bool {
        self.links.is_empty() && self.backlinks.is_empty()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Links(BTreeMap<PathBuf, FileLinks>);

impl Links {
    /// Create a new, empty Links struct
    pub fn new() -> Self {
        Self::default()
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
                    links: BTreeSet::from([to.clone()]),
                    backlinks: BTreeSet::new(),
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
                    links: BTreeSet::new(),
                    backlinks: BTreeSet::from([from]),
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
        // If there are no to_files, we still want to mark the `from` file as existing
        let iter = to_files.into_iter();
        let mut peekable = iter.peekable();
        if peekable.peek().is_none() {
            self.insert_file(from.clone());
        }
        for to in peekable {
            self.insert_link(from.clone(), to);
        }
    }

    /// Add a file that exists but has no outgoing links. This allows adding nodes into the graph
    /// that are orphans, or while manually constructing links.
    pub fn insert_file(&mut self, path: PathBuf) {
        let entry = self.0.entry(path).or_insert(FileLinks {
            exists: true,
            links: BTreeSet::new(),
            backlinks: BTreeSet::new(),
        });
        entry.exists = true;
    }

    /// Get the link info for a single file, if it exists
    pub fn get<Q: Borrow<PathBuf>>(&self, path: Q) -> Option<&FileLinks> {
        self.0.get(path.borrow())
    }

    /// Get an iterator over all files and their associated link info
    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &FileLinks)> {
        self.0.iter()
    }

    /// Get an iterator over all non-orphan files and their associated link info
    pub fn iter_non_orphans(&self) -> impl Iterator<Item = (&PathBuf, &FileLinks)> {
        self.0
            .iter()
            .filter(|(_, file_links)| !file_links.is_orphan())
    }

    /// Get an iterator over all orphan files. Because orphans have no links, this will only return
    /// file names
    pub fn iter_orphans(&self) -> impl Iterator<Item = &PathBuf> {
        self.0
            .iter()
            .filter_map(|(path, file_links)| file_links.is_orphan().then_some(path))
    }

    /// Prune all files that do not have any links or backlinks. This removes orphaned nodes from
    /// the graph.
    pub fn prune_orphans(&mut self) {
        self.0.retain(|_, file_links| !file_links.is_orphan());
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
        let mut visited = BTreeSet::new();
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
        let mut visited = BTreeSet::new();
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
    type IntoIter = std::collections::btree_map::IntoIter<PathBuf, FileLinks>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
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
    fn insert_links_without_targets_marks_source_file() {
        let mut links = Links::new();
        let lonely = PathBuf::from("/vault/orphan.md");

        links.insert_links(lonely.clone(), Vec::<PathBuf>::new());

        let entry = links.get(&lonely).expect("expected entry for lonely file");
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

        let visited: BTreeSet<PathBuf> = BTreeSet::from_iter(order);
        let expected = BTreeSet::from_iter([
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

        let visited: BTreeSet<PathBuf> = BTreeSet::from_iter(order);
        let expected = BTreeSet::from_iter([
            root.clone(),
            child_a.clone(),
            child_b.clone(),
            grandchild.clone(),
        ]);
        assert_eq!(visited, expected);
    }

    #[test]
    fn iter_orphans_only_returns_orphan_files() {
        let mut links = Links::new();
        let orphan_a = PathBuf::from("/vault/orphan-a.md");
        let orphan_b = PathBuf::from("/vault/orphan-b.md");
        let connected_a = PathBuf::from("/vault/linked-a.md");
        let connected_b = PathBuf::from("/vault/linked-b.md");

        links.insert_file(orphan_a.clone());
        links.insert_file(orphan_b.clone());
        links.insert_link(connected_a.clone(), connected_b.clone());

        let observed: BTreeSet<PathBuf> = links
            .iter_orphans()
            .cloned()
            .collect();
        let expected = BTreeSet::from_iter([orphan_a, orphan_b]);

        assert_eq!(observed, expected);
    }

    #[test]
    fn iter_non_orphans_excludes_orphan_files() {
        let mut links = Links::new();
        let orphan = PathBuf::from("/vault/orphan.md");
        let source = PathBuf::from("/vault/source.md");
        let target = PathBuf::from("/vault/target.md");

        links.insert_file(orphan);
        links.insert_link(source.clone(), target.clone());

        let observed: BTreeSet<PathBuf> = links
            .iter_non_orphans()
            .map(|(path, _)| path.clone())
            .collect();
        let expected = BTreeSet::from_iter([source, target]);

        assert_eq!(observed, expected);
    }

    #[test]
    fn prune_orphans_removes_orphan_entries() {
        let mut links = Links::new();
        let orphan = PathBuf::from("/vault/orphan.md");
        let source = PathBuf::from("/vault/source.md");
        let target = PathBuf::from("/vault/target.md");

        links.insert_file(orphan.clone());
        links.insert_link(source.clone(), target.clone());

        links.prune_orphans();

        assert!(links.get(&orphan).is_none(), "expected orphan to be pruned");
        assert!(links.get(&source).is_some(), "non-orphan should remain");
        assert!(links.get(&target).is_some(), "backlinked file should remain");
    }
}
