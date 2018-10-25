use buffer::{self, Point, Text};
use epoch::{self, Cursor, DirEntry, Epoch, FileId};
use futures::{future, stream, Async, Future, Poll, Stream};
use notify_cell::NotifyCell;
use std::cell::{Ref, RefCell, RefMut};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use time;
use Error;
use Oid;
use ReplicaId;

pub trait GitProvider {
    fn base_entries(&self, oid: Oid) -> Box<Stream<Item = DirEntry, Error = io::Error>>;
    fn base_text(&self, oid: Oid, path: &Path) -> Box<Future<Item = String, Error = io::Error>>;
}

pub struct WorkTree {
    epoch: Option<Rc<RefCell<Epoch>>>,
    buffers: Rc<RefCell<HashMap<BufferId, FileId>>>,
    next_buffer_id: BufferId,
    deferred_ops: Rc<RefCell<HashMap<epoch::Id, Vec<epoch::Operation>>>>,
    lamport_clock: Rc<RefCell<time::Lamport>>,
    git: Rc<GitProvider>,
    updates: NotifyCell<()>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Operation {
    StartEpoch {
        epoch_id: epoch::Id,
        head: Oid,
    },
    EpochOperation {
        epoch_id: epoch::Id,
        operation: epoch::Operation,
    },
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct BufferId(u32);

struct OpenTextFile {
    buffer_id: BufferId,
    path: PathBuf,
    epoch: Rc<RefCell<Epoch>>,
    git: Rc<GitProvider>,
    buffers: Rc<RefCell<HashMap<BufferId, FileId>>>,
    lamport_clock: Rc<RefCell<time::Lamport>>,
    state: OpenTextFileState,
}

enum OpenTextFileState {
    Start,
    Loading {
        file_id: FileId,
        epoch_id: epoch::Id,
        base_text_future: Box<Future<Item = String, Error = io::Error>>,
    },
}

impl WorkTree {
    pub fn new<I>(
        replica_id: ReplicaId,
        base: Oid,
        ops: I,
        git: Rc<GitProvider>,
    ) -> Result<(WorkTree, Box<Stream<Item = Operation, Error = Error>>), Error>
    where
        I: 'static + IntoIterator<Item = Operation>,
    {
        let mut ops = ops.into_iter().peekable();
        let mut tree = WorkTree {
            epoch: None,
            buffers: Rc::new(RefCell::new(HashMap::new())),
            next_buffer_id: BufferId(0),
            deferred_ops: Rc::new(RefCell::new(HashMap::new())),
            lamport_clock: Rc::new(RefCell::new(time::Lamport::new(replica_id))),
            git,
            updates: NotifyCell::new(()),
        };

        let ops = if ops.peek().is_none() {
            Box::new(tree.reset(base)) as Box<Stream<Item = Operation, Error = Error>>
        } else {
            Box::new(tree.apply_ops(ops)?) as Box<Stream<Item = Operation, Error = Error>>
        };

        Ok((tree, ops))
    }

    pub fn reset(&mut self, head: Oid) -> impl Stream<Item = Operation, Error = Error> {
        let epoch_id = self.lamport_clock.borrow_mut().tick();
        stream::once(Ok(Operation::StartEpoch { epoch_id, head }))
            .chain(self.start_epoch(epoch_id, head))
    }

    pub fn apply_ops<I>(
        &mut self,
        ops: I,
    ) -> Result<impl Stream<Item = Operation, Error = Error>, Error>
    where
        I: IntoIterator<Item = Operation>,
    {
        let mut cur_epoch_ops = Vec::new();
        let mut epoch_streams = Vec::new();

        for op in ops {
            match op {
                Operation::StartEpoch { epoch_id, head } => {
                    epoch_streams.push(self.start_epoch(epoch_id, head));
                }
                Operation::EpochOperation {
                    epoch_id,
                    operation,
                } => {
                    if let Some(epoch) = self.epoch.clone() {
                        match epoch_id.cmp(&epoch.borrow().id) {
                            Ordering::Less => {}
                            Ordering::Equal => cur_epoch_ops.push(operation),
                            Ordering::Greater => self.defer_epoch_op(epoch_id, operation),
                        }
                    } else {
                        self.defer_epoch_op(epoch_id, operation);
                    }
                }
            }
        }

        if let Some(epoch) = self.epoch.clone() {
            let mut epoch = epoch.borrow_mut();
            let fixup_ops = epoch.apply_ops(cur_epoch_ops, &mut self.lamport_clock.borrow_mut())?;
            let fixup_ops_stream = Box::new(stream::iter_ok(Operation::stamp(epoch.id, fixup_ops)));
            Ok(epoch_streams.into_iter().fold(
                fixup_ops_stream as Box<Stream<Item = Operation, Error = Error>>,
                |acc, stream| Box::new(acc.chain(stream)),
            ))
        } else {
            Err(Error::InvalidOperations)
        }
    }

    fn start_epoch(
        &mut self,
        epoch_id: epoch::Id,
        head: Oid,
    ) -> Box<Stream<Item = Operation, Error = Error>> {
        if self
            .epoch
            .as_ref()
            .map_or(true, |e| epoch_id > e.borrow().id)
        {
            let epoch = Rc::new(RefCell::new(Epoch::new(self.replica_id(), epoch_id, head)));
            if self.epoch.is_none() {
                self.epoch = Some(epoch.clone());
            }
            let cur_epoch = self.epoch.clone().unwrap();
            let deferred_ops = self.deferred_ops.clone();
            let lamport_clock_1 = self.lamport_clock.clone();
            let lamport_clock_2 = self.lamport_clock.clone();

            let epoch_1 = epoch.clone();
            let load_base_entries = self
                .git
                .base_entries(head)
                .map_err(|err| Error::IoError(err))
                .chunks(500)
                .and_then(move |base_entries| {
                    let fixup_ops = epoch_1
                        .borrow_mut()
                        .append_base_entries(base_entries, &mut lamport_clock_1.borrow_mut())?;
                    Ok(stream::iter_ok(Operation::stamp(epoch_id, fixup_ops)))
                })
                .flatten();

            let epoch_2 = epoch.clone();
            let assign_epoch = future::lazy(move || {
                let mut fixup_ops = Vec::new();
                if epoch_id > cur_epoch.borrow().id {
                    cur_epoch.swap(epoch_2.as_ref());
                    if let Some(ops) = deferred_ops.borrow_mut().remove(&epoch_id) {
                        fixup_ops = cur_epoch
                            .borrow_mut()
                            .apply_ops(ops, &mut lamport_clock_2.borrow_mut())?;
                    }
                    deferred_ops.borrow_mut().retain(|id, _| *id > epoch_id);
                }

                Ok(Box::new(stream::iter_ok(Operation::stamp(
                    epoch_id, fixup_ops,
                ))))
            })
            .flatten_stream();

            Box::new(load_base_entries.chain(assign_epoch))
        } else {
            Box::new(stream::empty())
        }
    }

    pub fn version(&self) -> time::Global {
        self.cur_epoch().version()
    }

    pub fn with_cursor<F>(&self, mut f: F)
    where
        F: FnMut(&mut Cursor),
    {
        if let Some(mut cursor) = self.cur_epoch().cursor() {
            f(&mut cursor);
        }
    }

    pub fn new_text_file(&mut self) -> (FileId, Operation) {
        let mut cur_epoch = self.cur_epoch_mut();
        let (file_id, operation) = cur_epoch.new_text_file(&mut self.lamport_clock.borrow_mut());
        (
            file_id,
            Operation::EpochOperation {
                epoch_id: cur_epoch.id,
                operation,
            },
        )
    }

    pub fn create_dir<N>(&mut self, path: &Path) -> Result<(FileId, Operation), Error>
    where
        N: AsRef<OsStr>,
    {
        let name = path
            .file_name()
            .ok_or(Error::InvalidPath("path has no file name".into()))?;
        let mut cur_epoch = self.cur_epoch_mut();
        let parent_id = if let Some(parent_path) = path.parent() {
            cur_epoch.file_id(parent_path)?
        } else {
            epoch::ROOT_FILE_ID
        };
        let epoch_id = cur_epoch.id;
        let (file_id, operation) =
            cur_epoch.create_dir(parent_id, name, &mut self.lamport_clock.borrow_mut())?;
        Ok((
            file_id,
            Operation::EpochOperation {
                epoch_id,
                operation,
            },
        ))
    }

    pub fn open_text_file<T>(
        &mut self,
        path: PathBuf,
    ) -> Box<Future<Item = BufferId, Error = Error>>
    where
        T: Into<Text>,
    {
        if let Some(buffer_id) = self.existing_buffer(&path) {
            Box::new(future::ok(buffer_id))
        } else {
            let buffer_id = self.next_buffer_id();
            Box::new(OpenTextFile {
                buffer_id,
                path,
                epoch: self.epoch.as_ref().unwrap().clone(),
                git: self.git.clone(),
                buffers: self.buffers.clone(),
                lamport_clock: self.lamport_clock.clone(),
                state: OpenTextFileState::Start,
            })
        }
    }

    fn existing_buffer(&self, path: &Path) -> Option<BufferId> {
        let cur_epoch = self.cur_epoch();
        for (buffer_id, file_id) in self.buffers.borrow().iter() {
            if let Some(existing_path) = cur_epoch.path(*file_id) {
                if path == existing_path {
                    return Some(*buffer_id);
                }
            }
        }
        None
    }

    pub fn rename<N>(&self, old_path: &Path, new_path: &Path) -> Result<Operation, Error>
    where
        N: AsRef<OsStr>,
    {
        let mut cur_epoch = self.cur_epoch_mut();
        let file_id = cur_epoch.file_id(old_path)?;
        let new_name = new_path
            .file_name()
            .ok_or(Error::InvalidPath("new path has no file name".into()))?;
        let new_parent_id = if let Some(parent_path) = new_path.parent() {
            cur_epoch.file_id(parent_path)?
        } else {
            epoch::ROOT_FILE_ID
        };

        let epoch_id = cur_epoch.id;
        let operation = cur_epoch.rename(
            file_id,
            new_parent_id,
            new_name,
            &mut self.lamport_clock.borrow_mut(),
        )?;
        Ok(Operation::EpochOperation {
            epoch_id,
            operation,
        })
    }

    pub fn remove(&self, path: &Path) -> Result<Operation, Error> {
        let mut cur_epoch = self.cur_epoch_mut();
        let file_id = cur_epoch.file_id(path)?;
        let epoch_id = cur_epoch.id;
        let operation = cur_epoch.remove(file_id, &mut self.lamport_clock.borrow_mut())?;

        Ok(Operation::EpochOperation {
            epoch_id,
            operation,
        })
    }

    pub fn edit<I, T>(
        &self,
        buffer_id: BufferId,
        old_ranges: I,
        new_text: T,
    ) -> Result<Operation, Error>
    where
        I: IntoIterator<Item = Range<usize>>,
        T: Into<Text>,
    {
        let file_id = self.buffer_file_id(buffer_id)?;
        let mut cur_epoch = self.cur_epoch_mut();
        let epoch_id = cur_epoch.id;
        let operation = cur_epoch
            .edit(
                file_id,
                old_ranges,
                new_text,
                &mut self.lamport_clock.borrow_mut(),
            )
            .unwrap();

        Ok(Operation::EpochOperation {
            epoch_id,
            operation,
        })
    }

    pub fn edit_2d<I, T>(
        &self,
        buffer_id: BufferId,
        old_ranges: I,
        new_text: T,
    ) -> Result<Operation, Error>
    where
        I: IntoIterator<Item = Range<Point>>,
        T: Into<Text>,
    {
        let file_id = self.buffer_file_id(buffer_id)?;
        let mut cur_epoch = self.cur_epoch_mut();
        let epoch_id = cur_epoch.id;
        let operation = cur_epoch
            .edit_2d(
                file_id,
                old_ranges,
                new_text,
                &mut self.lamport_clock.borrow_mut(),
            )
            .unwrap();

        Ok(Operation::EpochOperation {
            epoch_id,
            operation,
        })
    }

    pub fn path(&self, buffer_id: BufferId) -> Option<PathBuf> {
        self.buffers
            .borrow()
            .get(&buffer_id)
            .and_then(|file_id| self.cur_epoch().path(*file_id))
    }

    pub fn text(&self, buffer_id: BufferId) -> Result<buffer::Iter, Error> {
        let file_id = self.buffer_file_id(buffer_id)?;
        self.cur_epoch().text(file_id)
    }

    pub fn changes_since(
        &self,
        buffer_id: BufferId,
        version: time::Global,
    ) -> Result<impl Iterator<Item = buffer::Change>, Error> {
        let file_id = self.buffer_file_id(buffer_id)?;
        self.cur_epoch().changes_since(file_id, version)
    }

    fn cur_epoch(&self) -> Ref<Epoch> {
        self.epoch.as_ref().unwrap().borrow()
    }

    fn cur_epoch_mut(&self) -> RefMut<Epoch> {
        self.epoch.as_ref().unwrap().borrow_mut()
    }

    fn defer_epoch_op(&self, epoch_id: epoch::Id, operation: epoch::Operation) {
        self.deferred_ops
            .borrow_mut()
            .entry(epoch_id)
            .or_insert(Vec::new())
            .push(operation);
    }

    fn replica_id(&self) -> ReplicaId {
        self.lamport_clock.borrow().replica_id
    }

    fn parent_id<'a>(&self, epoch: &Epoch, path: &'a Path) -> Result<FileId, Error> {
        if let Some(parent_path) = path.parent() {
            epoch.file_id(parent_path)
        } else {
            Ok(epoch::ROOT_FILE_ID)
        }
    }

    fn next_buffer_id(&mut self) -> BufferId {
        let buffer_id = self.next_buffer_id;
        self.next_buffer_id.0 += 1;
        buffer_id
    }

    fn buffer_file_id(&self, buffer_id: BufferId) -> Result<FileId, Error> {
        self.buffers
            .borrow()
            .get(&buffer_id)
            .cloned()
            .ok_or(Error::InvalidBufferId)
    }
}

impl Operation {
    fn stamp<T>(epoch_id: epoch::Id, operations: T) -> impl Iterator<Item = Operation>
    where
        T: IntoIterator<Item = epoch::Operation>,
    {
        operations
            .into_iter()
            .map(move |operation| Operation::EpochOperation {
                epoch_id,
                operation,
            })
    }
}

// This future is implemented as a hand-rolled state machine. If the path being opened corresponds
// to a *new* file in the current epoch, we can open the file immediately with an empty base text.
// If the file existed in the base commit, we use the GitProvider to load its base text
// asynchronously. When the base text is done loading, we check that the current epoch did not
// change during loading. If it didn't change, we proceed to open the buffer. If It did change, we
// return to the Start state and try again.
impl Future for OpenTextFile {
    type Item = BufferId;
    type Error = Error;

    fn poll(&mut self) -> Poll<BufferId, Error> {
        let mut epoch = self.epoch.borrow_mut();

        loop {
            let file_id;
            let mut base_text = None;
            let mut next_state = None;

            match &mut self.state {
                OpenTextFileState::Start => {
                    file_id = epoch.file_id(&self.path)?;
                    if let Some(base_path) = epoch.base_path(file_id) {
                        next_state = Some(OpenTextFileState::Loading {
                            file_id,
                            epoch_id: epoch.id,
                            base_text_future: self.git.base_text(epoch.head, &base_path),
                        });
                    } else {
                        base_text = Some("".to_owned());
                    }
                }
                OpenTextFileState::Loading {
                    file_id: loaded_file_id,
                    epoch_id,
                    base_text_future,
                } => {
                    file_id = *loaded_file_id;
                    if epoch.id == *epoch_id {
                        match base_text_future.poll() {
                            Ok(Async::Ready(text)) => base_text = Some(text),
                            Ok(Async::NotReady) => return Ok(Async::NotReady),
                            Err(error) => return Err(Error::IoError(error)),
                        }
                    } else {
                        next_state = Some(OpenTextFileState::Start);
                    }
                }
            }

            if let Some(next_state) = next_state {
                self.state = next_state;
            }

            if let Some(base_text) = base_text {
                epoch.open_text_file(
                    file_id,
                    base_text.as_str(),
                    &mut self.lamport_clock.borrow_mut(),
                )?;
                self.buffers.borrow_mut().insert(self.buffer_id, file_id);
                return Ok(Async::Ready(self.buffer_id));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use epoch::CursorEntry;
    use rand::{SeedableRng, StdRng};

    #[test]
    fn test_reset() {
        let mut rng = StdRng::from_seed(&[1]);
        let mut base_tree_clock = &mut time::Lamport::new(999);

        let mut base_tree = Epoch::with_replica_id(999);
        base_tree.mutate(&mut rng, &mut base_tree_clock, 5);

        let mut git = TestGitProvider::new();
        git.commit([0; 20], base_tree.clone());

        base_tree.mutate(&mut rng, &mut base_tree_clock, 5);
        git.commit([1; 20], base_tree.clone());

        base_tree.mutate(&mut rng, &mut base_tree_clock, 5);
        git.commit([2; 20], base_tree.clone());

        let git = Rc::new(git);
        let (mut tree_1, ops_1) = WorkTree::new(1, [0; 20], Vec::new(), git.clone()).unwrap();
        let (mut tree_2, ops_2) =
            WorkTree::new(2, [0; 20], ops_1.collect().wait().unwrap(), git.clone()).unwrap();
        assert!(ops_2.wait().next().is_none());

        assert_eq!(tree_1.dir_entries(), git.tree([0; 20]).dir_entries());
        assert_eq!(tree_2.dir_entries(), git.tree([0; 20]).dir_entries());

        let ops_1 = tree_1.reset([1; 20]).collect().wait().unwrap();
        assert_eq!(tree_1.dir_entries(), git.tree([1; 20]).dir_entries());

        let ops_2 = tree_2.reset([2; 20]).collect().wait().unwrap();
        assert_eq!(tree_2.dir_entries(), git.tree([2; 20]).dir_entries());

        let fixup_ops_1 = tree_1.apply_ops(ops_2).unwrap().collect().wait().unwrap();
        let fixup_ops_2 = tree_2.apply_ops(ops_1).unwrap().collect().wait().unwrap();
        assert!(fixup_ops_1.is_empty());
        assert!(fixup_ops_2.is_empty());
        assert_eq!(tree_1.entries(), tree_2.entries());
    }

    impl WorkTree {
        fn entries(&self) -> Vec<CursorEntry> {
            self.epoch.as_ref().unwrap().borrow().entries()
        }

        fn dir_entries(&self) -> Vec<DirEntry> {
            self.epoch.as_ref().unwrap().borrow().dir_entries()
        }
    }

    struct TestGitProvider {
        commits: HashMap<Oid, Epoch>,
    }

    impl TestGitProvider {
        fn new() -> Self {
            TestGitProvider {
                commits: HashMap::new(),
            }
        }

        fn commit(&mut self, oid: Oid, tree: Epoch) {
            self.commits.insert(oid, tree);
        }

        fn tree(&self, oid: Oid) -> &Epoch {
            self.commits.get(&oid).unwrap()
        }
    }

    impl GitProvider for TestGitProvider {
        fn base_entries(&self, oid: Oid) -> Box<Stream<Item = DirEntry, Error = io::Error>> {
            Box::new(stream::iter_ok(
                self.commits
                    .get(&oid)
                    .unwrap()
                    .entries()
                    .into_iter()
                    .map(|entry| entry.into()),
            ))
        }

        fn base_text(
            &self,
            oid: Oid,
            path: &Path,
        ) -> Box<Future<Item = String, Error = io::Error>> {
            unimplemented!()
        }
    }
}
