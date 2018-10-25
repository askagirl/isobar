#![feature(macros_in_extern)]

extern crate bincode;
extern crate futures;
extern crate hex;
extern crate js_sys;
extern crate nano_core;
#[macro_use]
extern crate serde_derive;
extern crate base64;
extern crate serde;
extern crate wasm_bindgen;
extern crate wasm_bindgen_futures;

use futures::{Async, Future, Poll, Stream};
use nano_core as nano;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cell::Cell;
use std::io;
use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, JsFuture};

#[wasm_bindgen]
pub struct WorkTree(nano::WorkTree);

#[derive(Serialize, Deserialize)]
struct AsyncResult<T> {
    value: Option<T>,
    done: bool,
}

#[wasm_bindgen(module = "./support")]
extern "C" {
    pub type AsyncIteratorWrapper;

    #[wasm_bindgen(method)]
    fn next(this: &AsyncIteratorWrapper) -> js_sys::Promise;

    pub type GitProviderWrapper;

    #[wasm_bindgen(method, js_name = baseEntries)]
    fn base_entries(this: &GitProviderWrapper, head: &str) -> AsyncIteratorWrapper;
}

struct AsyncIteratorToStream<T, E> {
    next_value: JsFuture,
    iterator: AsyncIteratorWrapper,
    _phantom: PhantomData<(T, E)>,
}

#[wasm_bindgen]
pub struct StreamToAsyncIterator(Rc<Cell<Option<Box<Stream<Item = JsValue, Error = JsValue>>>>>);

struct HexOid(nano::Oid);

struct Base64<T>(T);

#[derive(Deserialize)]
pub struct WorkTreeNewArgs {
    replica_id: nano::ReplicaId,
    base: HexOid,
    start_ops: Vec<Base64<nano::Operation>>,
}

#[wasm_bindgen]
pub struct WorkTreeNewResult {
    tree: Option<WorkTree>,
    operations: Option<StreamToAsyncIterator>,
}

#[derive(Serialize)]
pub struct WorkTreeNewTextFileResult {
    file_id: Base64<nano::FileId>,
    operation: Base64<nano::Operation>,
}

#[wasm_bindgen]
impl WorkTree {
    pub fn new(git: GitProviderWrapper, args: JsValue) -> Result<WorkTreeNewResult, JsValue> {
        let WorkTreeNewArgs {
            replica_id,
            base: HexOid(base),
            start_ops,
        } = args.into_serde().unwrap();
        let (tree, operations) = nano::WorkTree::new(
            replica_id,
            base,
            start_ops.into_iter().map(|op| op.0),
            Rc::new(git),
        ).map_err(|e| e.to_string())?;
        Ok(WorkTreeNewResult {
            tree: Some(WorkTree(tree)),
            operations: Some(StreamToAsyncIterator::new(
                operations
                    .map(|op| Base64(op))
                    .map_err(|err| err.to_string()),
            )),
        })
    }

    pub fn new_text_file(&mut self) -> JsValue {
        let (file_id, operation) = self.0.new_text_file();
        JsValue::from_serde(&WorkTreeNewTextFileResult {
            file_id: Base64(file_id),
            operation: Base64(operation),
        }).unwrap()
    }

    pub fn open_text_file(&mut self, file_id: JsValue) -> js_sys::Promise {
        let Base64(file_id) = file_id.into_serde().unwrap();
        self.0.open_text_file(file_id)
    }
}

#[wasm_bindgen]
impl WorkTreeNewResult {
    pub fn tree(&mut self) -> WorkTree {
        self.tree.take().unwrap()
    }

    pub fn operations(&mut self) -> StreamToAsyncIterator {
        self.operations.take().unwrap()
    }
}

impl<T, E> AsyncIteratorToStream<T, E> {
    fn new(iterator: AsyncIteratorWrapper) -> Self {
        AsyncIteratorToStream {
            next_value: JsFuture::from(iterator.next()),
            iterator,
            _phantom: PhantomData,
        }
    }
}

impl<T, E> Stream for AsyncIteratorToStream<T, E>
where
    E: for<'de> Deserialize<'de>,
    T: for<'de> Deserialize<'de>,
{
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.next_value.poll() {
            Ok(Async::Ready(result)) => {
                let result: AsyncResult<T> = result.into_serde().unwrap();
                if result.done {
                    Ok(Async::Ready(None))
                } else {
                    self.next_value = JsFuture::from(self.iterator.next());
                    Ok(Async::Ready(result.value))
                }
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(error) => Err(error.into_serde().unwrap()),
        }
    }
}

impl StreamToAsyncIterator {
    fn new<E, S, T>(stream: S) -> Self
    where
        E: Serialize,
        S: 'static + Stream<Item = T, Error = E>,
        T: Serialize,
    {
        let js_value_stream = stream
            .map(|value| {
                JsValue::from_serde(&AsyncResult {
                    value: Some(value),
                    done: false,
                }).unwrap()
            }).map_err(|error| JsValue::from_serde(&error).unwrap());

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
                    Ok(next.unwrap_or(
                        JsValue::from_serde(&AsyncResult::<()> {
                            value: None,
                            done: true,
                        }).unwrap(),
                    ))
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
        unimplemented!()
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
        let bytes = hex::decode(&String::deserialize(deserializer)?).map_err(Error::custom)?;
        let mut oid = nano::Oid::default();
        oid.copy_from_slice(&bytes);
        Ok(HexOid(oid))
    }
}

impl<T: Serialize> Serialize for Base64<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        base64::encode(&bincode::serialize(&self.0).map_err(Error::custom)?).serialize(serializer)
    }
}

impl<'de1, T: for<'de2> Deserialize<'de2>> Deserialize<'de1> for Base64<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de1>,
    {
        use serde::de::Error;
        let bytes = base64::decode(&String::deserialize(deserializer)?).map_err(Error::custom)?;
        let inner = bincode::deserialize::<T>(&bytes).map_err(D::Error::custom)?;
        Ok(Base64(inner))
    }
}
