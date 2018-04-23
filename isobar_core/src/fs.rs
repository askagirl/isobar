use parking_lot::RwLock;
use std::ffi::{OsString, OsStr};
use std::path::{Path, PathBuf};
use std::result;
use std::sync::{Arc, Weak};
use std::iter::Iterator;
use futures::{Async, Poll, Stream};
use std::os::unix::ffi::OsStrExt;
use std::u16;
use std::usize;
use std::cmp::Ordering;

pub type Result<T> = result::Result<T, ()>;
type Entries = Vec<(OsString, Entry)>;

pub trait Tree {
    fn path(&self) -> &Path;
    fn root(&self) -> &Entry;
    fn updates(&self) -> Box<Stream<Item = (), Error = ()>>;
}

#[derive(Clone, Debug)]
pub enum Entry {
    Dir(Arc<RwLock<DirInner>>),
    File(Arc<FileInner>),
}

#[derive(Clone, Debug)]
pub struct DirInner {
    entries: Arc<Entries>,
    is_symlink: bool,
}

#[derive(Clone, Debug)]
pub struct FileInner {
    is_symlink: bool,
}

pub struct Search {
    query: Vec<char>,
    max_results: usize,
    results: Vec<SearchResult>,
    parent_path: PathBuf,
    stack: Vec<StackEntry>,
    match_variants: Vec<MatchVariant>,
    entry_count_per_poll: usize,
    done: bool,
    handle_ref: Weak<()>,
}

pub struct SearchHandle(Arc<()>);

#[derive(Clone, Debug)]
pub struct SearchResult {
    path: PathBuf,
    score: i64,
    match_indices: Vec<u16>,
}

struct StackEntry {
    entries: Arc<Entries>,
    entries_index: usize,
    prev_variants_len: usize,
}

#[derive(Clone, PartialEq, Eq)]
struct MatchVariant {
    query_index: u16,
    score: i64,
    match_indices: Vec<u16>,
}

impl Entry {
    pub fn file(is_symlink: bool) -> Self {
        Entry::File(Arc::new(FileInner { is_symlink }))
    }

    pub fn dir(is_symlink: bool) -> Self {
        Entry::Dir(Arc::new(RwLock::new(DirInner {
            entries: Arc::new(Vec::new()),
            is_symlink,
        })))
    }

    pub fn insert<T: Into<OsString>>(&self, new_name: T, new_entry: Entry) -> Result<()> {
        match self {
            &Entry::Dir(ref inner) => {
                let new_name = new_name.into();

                let mut inner = inner.write();
                let entries = Arc::make_mut(&mut inner.entries);
                if entries
                    .last()
                    .map(|&(ref name, _)| name < &new_name)
                    .unwrap_or(true)
                {
                    entries.push((new_name, new_entry));
                    Ok(())
                } else {
                    match entries.binary_search_by(|&(ref name, _)| name.cmp(&new_name)) {
                        Ok(_) => Err(()), // An entry already exists with this name
                        Err(index) => {
                            entries.insert(index, (new_name, new_entry));
                            Ok(())
                        }
                    }
                }
            }
            &Entry::File(_) => Err(()),
        }
    }
}

impl Stream for Search {
    type Item = Vec<SearchResult>;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if self.done || self.handle_ref.upgrade().is_none() {
            return Ok(Async::Ready(None));
        }

        for _ in 0..self.entry_count_per_poll {
            let prev_variants_len = self.match_variants.len();

            enum Action {
                Push(StackEntry),
                Pop,
                UpdateResults(OsString),
            };

            let action: Action;
            if let Some(ref mut cur_dir) = self.stack.last_mut() {
                if cur_dir.entries_index < cur_dir.entries.len() {
                    let child = &cur_dir.entries[cur_dir.entries_index];
                    Self::update_match_variants(
                        &mut self.match_variants,
                        &self.query,
                        &self.parent_path,
                        &child.0
                    );
                    match child.1 {
                        Entry::Dir(ref inner) => {
                            self.parent_path.push(&child.0);
                            action = Action::Push(StackEntry {
                                entries: inner.read().entries.clone(),
                                entries_index: 0,
                                prev_variants_len: self.match_variants.len(),
                            });
                        },
                        Entry::File(_) => {
                            cur_dir.entries_index += 1;
                            action = Action::UpdateResults(child.0.clone());
                        }
                    }
                } else {
                    action = Action::Pop;
                }
            } else {
                return Ok(Async::Ready(Some(self.results.clone())));
            }

            match action {
                Action::Push(new_stack_entry) => {
                    self.stack.push(new_stack_entry);
                },
                Action::Pop => {
                    let last = self.stack.pop().unwrap();
                    self.match_variants.truncate(last.prev_variants_len);
                    if let Some(mut last) = self.stack.last_mut() {
                        last.entries_index += 1;
                    }
                    self.parent_path.pop();
                },
                Action::UpdateResults(filename) => {
                    self.update_results(filename);
                    self.match_variants.truncate(prev_variants_len);
                },
            }
        }

        self.done = true;
        return Ok(Async::Ready(Some(self.results.clone())));
    }
}

const SUBWORD_START_BONUS: i64 = 10;
const CONSECUTIVE_BONUS: i64 = 5;
const LEADING_MISMATCH_LENGTH: u16 = 3;
const LEADING_MISMATCH_PENALTY: i64 = 3;
const MISMATCH_PENALTY: i64 = 1;

impl Search {
    pub fn new(entry: &Entry, query: &str, max_results: usize) -> (Self, SearchHandle) {
        match entry {
            &Entry::Dir(ref inner) => {
                let handle = SearchHandle(Arc::new(()));
                let search = Search {
                    query: query.chars().map(|c| c.to_ascii_lowercase()).collect(),
                    max_results,
                    results: Vec::new(),
                    parent_path: PathBuf::new(),
                    stack: vec![StackEntry {
                        entries: inner.read().entries.clone(),
                        entries_index: 0,
                        prev_variants_len: 0,
                    }],
                    match_variants: vec![MatchVariant {
                        score: 0,
                        query_index: 0,
                        match_indices: Vec::new(),
                    }],
                    done: false,
                    entry_count_per_poll: usize::MAX,
                    handle_ref: Arc::downgrade(&handle.0),
                };
                (search, handle)
            },
            _ => panic!("search_tree called on a file")
        }
    }

    pub fn set_entry_count_per_poll(&mut self, entry_count_per_poll: usize) -> &mut Self {
        self.entry_count_per_poll = entry_count_per_poll;
        self
    }

    fn update_results(&mut self, filename: OsString) {
        for variant in self.match_variants.iter().rev() {
            if variant.query_index == self.query.len() as u16 {
                let search_result = self.results.binary_search_by(|result| {
                    if result.score > variant.score {
                        Ordering::Less
                    } else if result.score < variant.score {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                });

                match search_result {
                    Ok(_) => {},
                    Err(index) => {
                        if index < self.max_results {
                            let mut path = self.parent_path.clone();
                            path.push(filename);
                            self.results.insert(index, SearchResult {
                                score: variant.score,
                                match_indices: variant.match_indices.clone(),
                                path,
                            });
                            self.results.truncate(self.max_results);
                            return;
                        }
                    }
                }
            }
        }
    }

    fn update_match_variants(match_variants: &mut Vec<MatchVariant>, query: &Vec<char>, parent_path: &PathBuf, name: &OsStr) {
        let mut new_variants = Vec::<MatchVariant>::new();

        let previous_character: char = '\0';
        for (name_index, character) in name.as_bytes().iter().map(|c| *c as char).enumerate() {
            let name_index = (name_index + parent_path.as_os_str().as_bytes().len() + 1) as u16;
            let character = character.to_ascii_lowercase();

            let mut i = 0;
            while i != match_variants.len() {
                let mut variant = &mut match_variants[i];
                if variant.query_index as usize == query.len() {
                    i += 1;
                    continue;
                }

                // If the current word character matches the next character of the query
                // for this match variant, create a new match variant that consumes the
                // matching character.
                let query_character = query[variant.query_index as usize];
                if character == query_character {
                    let mut new_variant = variant.clone();
                    new_variant.query_index += 1;

                    // Apply a bonus if the current character is the start of a word.
                    if !previous_character.is_alphanumeric() {
                        new_variant.score += SUBWORD_START_BONUS;
                    }

                    // Apply a bonus if the last character of the path also matched.
                    if new_variant.match_indices.last().map_or(false, |index| *index == name_index - 1) {
                        new_variant.score += CONSECUTIVE_BONUS;
                    }

                    new_variant.match_indices.push(name_index as u16);
                    new_variants.push(new_variant);
                }

                // For the current match variant, treat the current character as a mismatch
                // regardless of whether it matched above. This reserves the chance for the
                // next character to be consumed by a match with higher overall value.
                if name_index < LEADING_MISMATCH_LENGTH {
                  variant.score -= LEADING_MISMATCH_PENALTY;
                } else {
                  variant.score -= MISMATCH_PENALTY;
                }

                i += 1;
            }

            // Add all of the newly-computed match variants to the list. Avoid adding multiple
            // match variants with the same query index.
            let mut previous_query_index = u16::MAX;
            new_variants.sort_unstable_by(|a, b| {
                if a.query_index < b.query_index {
                    Ordering::Less
                } else if a.query_index > b.query_index {
                    Ordering::Greater
                } else if a.score > b.score {
                    Ordering::Less
                } else if a.score < b.score {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            });

            for new_variant in new_variants.drain(..) {
                if new_variant.query_index != previous_query_index {
                    previous_query_index = new_variant.query_index;
                    match_variants.push(new_variant);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Entry {
        fn entry_names(&self) -> Vec<String> {
            match self {
                &Entry::Dir(ref inner) => inner
                    .read()
                    .entries
                    .iter()
                    .map(|&(ref name, _)| name.clone().into_string().unwrap())
                    .collect(),
                _ => panic!(),
            }
        }
    }

    #[test]
    fn test_insert() {
        let root = Entry::dir(false);
        assert_eq!(root.insert("a", Entry::file(false)), Ok(()));
        assert_eq!(root.insert("c", Entry::file(false)), Ok(()));
        assert_eq!(root.insert("b", Entry::file(false)), Ok(()));
        assert_eq!(root.insert("a", Entry::file(false)), Err(()));
        assert_eq!(root.entry_names(), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_search() {
        let root = Entry::dir(false);
        let child1 = Entry::dir(false);
        let child2 = Entry::dir(false);
        let child1_child1 = Entry::file(false);
        let child1_child2 = Entry::file(false);
        let child2_child1 = Entry::file(false);
        let child2_child2 = Entry::file(false);

        child1.insert("egg", child1_child1);
        child1.insert("fox", child1_child2);
        child2.insert("gum", child2_child1);
        child2.insert("hut", child2_child2);
        root.insert("catsup", child1);
        root.insert("dog", child2);

        let (mut search, handle) = Search::new(&root, "eg", 10);
        search.set_entry_count_per_poll(50);

        let results = search.poll();

        match results {
            Ok(Async::Ready(Some(results))) => {
                assert_eq!(results[0].path, Path::new("catsup/egg"));
            },
            _ => panic!("Unexpected results {:?}", results)
        }
    }
}
