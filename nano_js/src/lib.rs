#![feature(macros_in_extern)]

use bincode;
use futures::{Async, Future, Poll, Stream};
use nano_core as nano;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_derive::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::HashSet;
use std::io;
use std::marker::PhantomData;
use std::mem;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::{future_to_promise, JsFuture};

trait JsValueExt {
    fn into_operation(self) -> Result<Option<nano::Operation>, JsValue>;
    fn into_error_message(self) -> Result<String, String>;
}

trait IntoJsError {
    fn into_js_err(self) -> JsValue;
}

#[wasm_bindgen]
pub struct WorkTree(nano::WorkTree);

#[derive(Deserialize)]
struct AsyncResult<T> {
    value: Option<T>,
    done: bool,
}

struct AsyncIteratorToStream<T> {
    next_value: JsFuture,
    iterator: AsyncIteratorWrapper,
    _phantom: PhantomData<T>,
}

#[wasm_bindgen]
pub struct StreamToAsyncIterator(Rc<Cell<Option<Box<Stream<Item = JsValue, Error = JsValue>>>>>);

#[wasm_bindgen]
pub struct WorkTreeNewResult {
    tree: Option<WorkTree>,
    operations: Option<StreamToAsyncIterator>,
}

#[wasm_bindgen]
pub struct OperationEnvelope(nano::OperationEnvelope);

#[derive(Copy, Clone, Serialize, Deserialize)]
struct EditRange {
    start: nano::Point,
    end: nano::Point,
}

#[derive(Serialize)]
struct Change {
    start: nano::Point,
    end: nano::Point,
    text: String,
}

#[derive(Serialize)]
struct Entry {
    #[serde(rename = "type")]
    file_type: nano::FileType,
    depth: usize,
    name: String,
    path: String,
    status: nano::FileStatus,
    visible: bool,
}

pub struct HexOid(nano::Oid);

#[wasm_bindgen(module = "./support")]
extern "C" {
    pub type AsyncIteratorWrapper;

    #[wasm_bindgen(method)]
    fn next(this: &AsyncIteratorWrapper) -> js_sys::Promise;

    pub type GitProviderWrapper;

    #[wasm_bindgen(method, js_name = baseEntries)]
    fn base_entries(this: &GitProviderWrapper, head: &str) -> AsyncIteratorWrapper;

    #[wasm_bindgen(method, js_name = baseText)]
    fn base_text(this: &GitProviderWrapper, head: &str, path: &str) -> js_sys::Promise;

    pub type ChangeObserver;

    #[wasm_bindgen(method, js_name = textChanged)]
    fn text_changed(this: &ChangeObserver, buffer_id: JsValue, changes: JsValue);
}

#[wasm_bindgen]
impl WorkTree {
    pub fn new(
        git: GitProviderWrapper,
        observer: ChangeObserver,
        replica_id: JsValue,
        base: JsValue,
        js_start_ops: js_sys::Array,
    ) -> Result<WorkTreeNewResult, JsValue> {
        let replica_id = replica_id.into_serde().map_err(|e| {
            format!("ReplicaId {:?} must be a valid UUID: {}", replica_id, e).into_js_err()
        })?;

        let base = base
            .into_serde::<Option<HexOid>>()
            .map_err(|e| e.into_js_err())?
            .map(|b| b.0);

        let mut start_ops = Vec::new();
        for js_op in js_start_ops.values() {
            if let Some(op) = js_op?.into_operation()? {
                start_ops.push(op);
            }
        }

        let (tree, operations) = nano::WorkTree::new(
            replica_id,
            base,
            start_ops,
            Rc::new(git),
            Some(Rc::new(observer)),
        )
        .map_err(|e| e.into_js_err())?;
        Ok(WorkTreeNewResult {
            tree: Some(WorkTree(tree)),
            operations: Some(StreamToAsyncIterator::new(
                operations
                    .map(|op| JsValue::from(OperationEnvelope::new(op)))
                    .map_err(|e| e.into_js_err()),
            )),
        })
    }

    pub fn version(&self) -> Vec<u8> {
        bincode::serialize(&self.0.version()).unwrap()
    }

    pub fn observed(&self, version_bytes: &[u8]) -> Result<bool, JsValue> {
        let version = bincode::deserialize(&version_bytes).map_err(|e| e.into_js_err())?;
        Ok(self.0.observed(version))
    }

    pub fn head(&self) -> JsValue {
        JsValue::from_serde(&self.0.head().map(|head| HexOid(head))).unwrap()
    }

    pub fn reset(&mut self, base: JsValue) -> Result<StreamToAsyncIterator, JsValue> {
        let base = base
            .into_serde::<Option<HexOid>>()
            .map_err(|e| e.into_js_err())?
            .map(|b| b.0);
        Ok(StreamToAsyncIterator::new(
            self.0
                .reset(base)
                .map(|op| JsValue::from(OperationEnvelope::new(op)))
                .map_err(|e| e.into_js_err()),
        ))
    }

    pub fn apply_ops(&mut self, js_ops: js_sys::Array) -> Result<StreamToAsyncIterator, JsValue> {
        let mut ops = Vec::new();
        for js_op in js_ops.values() {
            if let Some(op) = js_op?.into_operation()? {
                ops.push(op);
            }
        }

        self.0
            .apply_ops(ops)
            .map(|fixup_ops| {
                StreamToAsyncIterator::new(
                    fixup_ops
                        .map(|op| JsValue::from(OperationEnvelope::new(op)))
                        .map_err(|e| e.into_js_err()),
                )
            })
            .map_err(|e| e.into_js_err())
    }

    pub fn create_file(
        &self,
        path: String,
        file_type: JsValue,
    ) -> Result<OperationEnvelope, JsValue> {
        let file_type = file_type.into_serde().map_err(|e| e.into_js_err())?;
        self.0
            .create_file(&path, file_type)
            .map(|operation| OperationEnvelope::new(operation))
            .map_err(|e| e.into_js_err())
    }

    pub fn rename(&self, old_path: String, new_path: String) -> Result<OperationEnvelope, JsValue> {
        self.0
            .rename(&old_path, &new_path)
            .map(|operation| OperationEnvelope::new(operation))
            .map_err(|e| e.into_js_err())
    }

    pub fn remove(&self, path: String) -> Result<OperationEnvelope, JsValue> {
        self.0
            .remove(&path)
            .map(|operation| OperationEnvelope::new(operation))
            .map_err(|e| e.into_js_err())
    }

    pub fn exists(&self, path: String) -> bool {
        self.0.exists(&path)
    }

    pub fn open_text_file(&mut self, path: String) -> js_sys::Promise {
        future_to_promise(
            self.0
                .open_text_file(path)
                .map(|buffer_id| JsValue::from_serde(&buffer_id).unwrap())
                .map_err(|e| e.into_js_err()),
        )
    }

    pub fn path(&self, buffer_id: JsValue) -> Result<Option<String>, JsValue> {
        let buffer_id = buffer_id.into_serde().map_err(|e| e.into_js_err())?;
        Ok(self
            .0
            .path(buffer_id)
            .map(|path| path.to_string_lossy().into_owned()))
    }

    pub fn text(&self, buffer_id: JsValue) -> Result<JsValue, JsValue> {
        let buffer_id = buffer_id.into_serde().map_err(|e| e.into_js_err())?;
        self.0
            .text(buffer_id)
            .map(|text| JsValue::from_str(&text.into_string()))
            .map_err(|e| e.into_js_err())
    }

    pub fn buffer_deferred_ops_len(&self, buffer_id: JsValue) -> Result<u32, JsValue> {
        let buffer_id = buffer_id.into_serde().map_err(|e| e.into_js_err())?;
        self.0
            .buffer_deferred_ops_len(buffer_id)
            .map(|len| len as u32)
            .map_err(|e| e.into_js_err())
    }

    pub fn edit(
        &self,
        buffer_id: JsValue,
        old_ranges: JsValue,
        new_text: &str,
    ) -> Result<OperationEnvelope, JsValue> {
        let buffer_id = buffer_id.into_serde().map_err(|e| e.into_js_err())?;
        let old_ranges = old_ranges
            .into_serde::<Vec<EditRange>>()
            .map_err(|e| e.into_js_err())?
            .into_iter()
            .map(|EditRange { start, end }| start..end);

        self.0
            .edit_2d(buffer_id, old_ranges, new_text)
            .map(|op| OperationEnvelope::new(op))
            .map_err(|e| e.into_js_err())
    }

    pub fn entries(&self, descend_into: JsValue, show_deleted: bool) -> Result<JsValue, JsValue> {
        let descend_into: Option<HashSet<PathBuf>> =
            descend_into.into_serde().map_err(|e| e.into_js_err())?;
        let mut entries = Vec::new();
        self.0.with_cursor(|cursor| loop {
            let entry = cursor.entry().unwrap();
            let mut descend = false;
            if show_deleted || entry.status != nano::FileStatus::Removed {
                let path = cursor.path().unwrap();
                entries.push(Entry {
                    file_type: entry.file_type,
                    depth: entry.depth,
                    name: entry.name.to_string_lossy().into_owned(),
                    path: path.to_string_lossy().into_owned(),
                    status: entry.status,
                    visible: entry.visible,
                });
                descend = descend_into.as_ref().map_or(true, |d| d.contains(path));
            }

            if !cursor.next(descend) {
                break;
            }
        });
        JsValue::from_serde(&entries).map_err(|e| e.into_js_err())
    }
}

#[wasm_bindgen]
impl WorkTreeNewResult {
    pub fn tree(&mut self) -> Result<WorkTree, JsValue> {
        self.tree
            .take()
            .ok_or(js_sys::Error::new("Cannot take tree twice").into())
    }

    pub fn operations(&mut self) -> Result<StreamToAsyncIterator, JsValue> {
        self.operations
            .take()
            .ok_or(js_sys::Error::new("Cannot take operations twice").into())
    }
}

#[wasm_bindgen]
impl OperationEnvelope {
    fn new(operation: nano::OperationEnvelope) -> Self {
        OperationEnvelope(operation)
    }

    #[wasm_bindgen(js_name = epochId)]
    pub fn epoch_id(&self) -> Vec<u8> {
        let epoch_id = self.0.operation.epoch_id();
        let timestamp_bytes: [u8; 8] = unsafe { mem::transmute(epoch_id.value.to_be()) };
        let mut epoch_id_bytes = Vec::with_capacity(24);
        epoch_id_bytes.extend_from_slice(&timestamp_bytes);
        epoch_id_bytes.extend_from_slice(epoch_id.replica_id.as_bytes());
        epoch_id_bytes
    }

    #[wasm_bindgen(js_name = epochReplicaId)]
    pub fn epoch_replica_id(&self) -> JsValue {
        JsValue::from_serde(&self.0.operation.epoch_id().replica_id).unwrap()
    }

    #[wasm_bindgen(js_name = epochTimestamp)]
    pub fn epoch_timestamp(&self) -> JsValue {
        JsValue::from_serde(&self.0.operation.epoch_id().value).unwrap()
    }

    #[wasm_bindgen(js_name = epochHead)]
    pub fn epoch_head(&self) -> JsValue {
        JsValue::from_serde(&self.0.epoch_head.map(|head| HexOid(head))).unwrap()
    }

    pub fn operation(&self) -> Vec<u8> {
        self.0.operation.serialize()
    }
}

impl<T> AsyncIteratorToStream<T> {
    fn new(iterator: AsyncIteratorWrapper) -> Self {
        AsyncIteratorToStream {
            next_value: JsFuture::from(iterator.next()),
            iterator,
            _phantom: PhantomData,
        }
    }
}

impl<T> Stream for AsyncIteratorToStream<T>
where
    T: for<'de> Deserialize<'de>,
{
    type Item = T;
    type Error = String;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.next_value.poll() {
            Ok(Async::Ready(result)) => {
                let result: AsyncResult<T> = result.into_serde().map_err(|e| e.to_string())?;
                if result.done {
                    Ok(Async::Ready(None))
                } else {
                    self.next_value = JsFuture::from(self.iterator.next());
                    Ok(Async::Ready(result.value))
                }
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(error) => Err(error.into_error_message()?),
        }
    }
}

impl StreamToAsyncIterator {
    fn new<S>(stream: S) -> Self
    where
        S: 'static + Stream<Item = JsValue, Error = JsValue>,
    {
        let js_value_stream = stream.map(|value| {
            let result = JsValue::from(js_sys::Object::new());
            js_sys::Reflect::set(&result, &JsValue::from_str("value"), &value).unwrap();
            js_sys::Reflect::set(
                &result,
                &JsValue::from_str("done"),
                &JsValue::from_bool(false),
            )
            .unwrap();
            result
        });

        StreamToAsyncIterator(Rc::new(Cell::new(Some(Box::new(js_value_stream)))))
    }
}

#[wasm_bindgen]
impl StreamToAsyncIterator {
    pub fn next(&mut self) -> Option<js_sys::Promise> {
        let stream_rc = self.0.clone();
        self.0.take().map(|stream| {
            future_to_promise(stream.into_future().then(move |result| match result {
                Ok((next, rest)) => {
                    stream_rc.set(Some(rest));
                    Ok(next.unwrap_or_else(|| {
                        let result = JsValue::from(js_sys::Object::new());
                        js_sys::Reflect::set(
                            &result,
                            &JsValue::from_str("done"),
                            &JsValue::from_bool(true),
                        )
                        .unwrap();
                        result
                    }))
                }
                Err((error, _)) => Err(error),
            }))
        })
    }
}

impl nano::GitProvider for GitProviderWrapper {
    fn base_entries(
        &self,
        oid: nano::Oid,
    ) -> Box<Stream<Item = nano::DirEntry, Error = io::Error>> {
        let iterator = GitProviderWrapper::base_entries(self, &hex::encode(oid));
        Box::new(
            AsyncIteratorToStream::new(iterator)
                .map_err(|error: String| io::Error::new(io::ErrorKind::Other, error)),
        )
    }

    fn base_text(
        &self,
        oid: nano::Oid,
        path: &Path,
    ) -> Box<Future<Item = String, Error = io::Error>> {
        Box::new(
            JsFuture::from(GitProviderWrapper::base_text(
                self,
                &hex::encode(oid),
                path.to_string_lossy().as_ref(),
            ))
            .then(|value| match value {
                Ok(value) => value
                    .as_string()
                    .ok_or_else(|| String::from("Text is not a string")),
                Err(error) => Err(error.into_error_message()?),
            })
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error)),
        )
    }
}

impl nano::ChangeObserver for ChangeObserver {
    fn text_changed(&self, buffer_id: nano::BufferId, changes: Box<Iterator<Item = nano::Change>>) {
        let changes = changes
            .map(|change| Change {
                start: change.range.start,
                end: change.range.end,
                text: String::from_utf16_lossy(&change.code_units),
            })
            .collect::<Vec<_>>();
        ChangeObserver::text_changed(
            self,
            JsValue::from_serde(&buffer_id).unwrap(),
            JsValue::from_serde(&changes).unwrap(),
        );
    }
}

impl Serialize for HexOid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        hex::encode(self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HexOid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let hex_string = String::deserialize(deserializer)?;
        let bytes = hex::decode(&hex_string).map_err(Error::custom)?;
        let mut oid = nano::Oid::default();
        if oid.len() == bytes.len() {
            oid.copy_from_slice(&bytes);
            Ok(HexOid(oid))
        } else {
            Err(D::Error::custom(format!(
                "{} cannot be parsed as a valid object id. pass a full 40-character hex string.",
                hex_string
            )))
        }
    }
}

impl<T: ToString> IntoJsError for T {
    fn into_js_err(self) -> JsValue {
        js_sys::Error::new(&self.to_string()).into()
    }
}

impl JsValueExt for JsValue {
    fn into_operation(self) -> Result<Option<nano::Operation>, JsValue> {
        let js_bytes = self
            .dyn_into::<js_sys::Uint8Array>()
            .map_err(|_| "Operation must be Uint8Array".into_js_err())?;
        let mut bytes = Vec::with_capacity(js_bytes.byte_length() as usize);
        js_bytes.for_each(&mut |byte, _, _| bytes.push(byte));
        nano::Operation::deserialize(&bytes).map_err(|e| e.into_js_err())
    }

    fn into_error_message(self) -> Result<String, String> {
        match self.dyn_into::<js_sys::Error>() {
            Ok(js_err) => Ok(js_err.message().into()),
            Err(_) => Err(String::from(
                "An error occurred but can't displayed because it's not an instance of an error",
            )),
        }
    }
}
