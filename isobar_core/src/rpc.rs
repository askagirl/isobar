use bincode::{deserialize, serialize};
use futures::stream::FuturesUnordered;
use futures::task::{self, Task};
use futures::{future, Async, Future, Poll, Stream};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io;
use std::mem;
use std::rc::Rc;

pub type RequestId = usize;
pub type ServiceId = usize;

pub trait Service {
    type State: 'static + Serialize;
    type Update: 'static + Serialize;
    type Request: 'static + for<'a> Deserialize<'a>;
    type Response: 'static + Serialize;
    type Error: 'static + Serialize;

    fn state(&self, connection: &mut ConnectionToClient) -> Self::State;
    fn poll_update(&mut self, connection: &mut ConnectionToClient) -> Async<Option<Self::Update>>;
    fn request(
        &mut self,
        _request: Self::Request,
        _connection: &mut ConnectionToClient,
    ) -> Option<Box<Future<Item = Self::Response, Error = Self::Error>>> {
        None
    }
}

trait RawBytesService {
    fn state(&self, connection: &mut ConnectionToClient) -> Vec<u8>;
    fn poll_update(&mut self, connection: &mut ConnectionToClient) -> Async<Option<Vec<u8>>>;
    fn request(
        &mut self,
        request: Vec<u8>,
        connection: &mut ConnectionToClient,
    ) -> Option<Box<Future<Item = Vec<u8>, Error = Vec<u8>>>>;
}

#[derive(Serialize, Deserialize)]
enum MessageToClient {
    Update {
        insertions: HashMap<ServiceId, Vec<u8>>,
        updates: HashMap<ServiceId, Vec<Vec<u8>>>,
        removals: HashSet<ServiceId>,
        responses: HashMap<ServiceId, Vec<(RequestId, Response)>>,
    },
    Err(String),
}

#[derive(Serialize, Deserialize)]
enum Response {
    Ok(Vec<u8>),
    Err(Vec<u8>),
    RpcErr(RpcError),
}

#[derive(Serialize, Deserialize)]
enum RpcError {
    ServiceNotFound,
}

#[derive(Serialize, Deserialize)]
enum MessageToServer {
    Request {
        service_id: ServiceId,
        request_id: RequestId,
        payload: Vec<u8>,
    },
}

pub struct ConnectionToClient {
    next_id: ServiceId,
    services: HashMap<ServiceId, Rc<RefCell<RawBytesService>>>,
    inserted: HashSet<ServiceId>,
    removed: HashSet<ServiceId>,
    incoming: Box<Stream<Item = Vec<u8>, Error = io::Error>>,
    pending_responses: FuturesUnordered<Box<Future<Item = ResponseEnvelope, Error = ()>>>,
    pending_task: Option<Task>,
}

struct ResponseEnvelope {
    service_id: ServiceId,
    request_id: RequestId,
    response: Response,
}

impl ConnectionToClient {
    pub fn new<S, T>(incoming: T, bootstrap: T) -> Self
    where
        S: 'static + Stream<Item = Vec<u8>, Error = io::Error>,
        T: 'static + Service,
    {
        let mut connection = Self {
            next_id: 0,
            services: HashMap::new(),
            inserted: HashSet::new(),
            removed: HashSet::new(),
            incoming: Box::new(incoming);
            pending_responses: FuturesUnordered::new(),
            pending_task: None,
        };
        connection.add_service(bootstrap);
        connection
    }

    pub fn add_service<T: 'static + Service>(&mut self, service: T) -> ServiceId {
        let id = self.next_id;
        self.next_id += 1;
        self.services.insert(id, Rc::new(RefCell::new(service)));
        self.inserted.insert(id);
        id
    }

    fn poll_incoming(&mut self) -> Result<bool, io::Error> {
        loop {
            match self.incoming.poll() {
                Ok(Async::Ready(Some(request))) => match deserialize(&request).unwrap() {
                    MessageToServer::Request {
                        request_id,
                        service_id,
                        payload,
                    } => {
                        if let Some(service) = self.services.get(&service_id).cloned() {
                            if let Some(response) = service.borrow_mut().request(payload, self) {
                                self.pending_responses
                                    .push(Box::new(response.then(move |response| {
                                        Ok(ResponseEnvelope {
                                            request_id,
                                            service_id,
                                            response: match response {
                                                Ok(payload) => Response::Ok(payload),
                                                Err(payload) => Response::Err(payload),
                                            },
                                        })
                                    })));
                            }
                        } else {
                            self.pending_responses
                                .push(Box::new(future::ok(ResponseEnvelope {
                                    request_id,
                                    service_id,
                                    response: Response::RpcErr(RpcError::ServiceNotFound),
                                })));
                        }
                    }
                },
                Ok(Async::Ready(None)) => return Ok(false),
                Ok(Async::NotReady) => return Ok(true),
                Err(error) => {
                    eprintln!("Error polling incoming connection: {}", error);
                    return Err(error);
                }
            }
        }
    }

    fn poll_outgoing(&mut self) -> Poll<Option<Vec<u8>>, ()> {
        let mut insertions = HashMap::new();
        let mut inserted = HashSet::new();
        mem::swap(&mut inserted, &mut self.inserted);
        for id in &inserted {
            if let Some(service) = self.services.get(id).cloned() {
                insertions.insert(*id, service.borrow().state(self));
            }
        }
        let mut updates: HashMap<ServiceId, Vec<Vec<u8>>> = HashMap::new();
        let service_ids = self.services.keys().cloned().collect::<Vec<ServiceId>>();
        for id in service_ids {
            let service = self.services.get(&id).unwrap().clone();
            let mut service_borrow = service.borrow_mut();
            loop {
                match service_borrow.poll_update(self) {
                    Async::Ready(Some(update)) => {
                        if !inserted.contains(&id) {
                            updates.entry(id).or_insert(Vec::new()).push(update);
                        }
                    }
                    Async::Ready(None) => unimplemented!("Terminate the service"),
                    Async::NotReady => break,
                }
            }
        }

        let mut removals = HashSet::new();
        mem::swap(&mut removals, &mut self.removed);

        let mut responses = HashMap::new();
        loop {
            match self.pending_responses.poll() {
                Ok(Async::Ready(Some(envelope))) => {
                    responses
                        .entry(envelope.service_id)
                        .or_insert(Vec::new())
                        .push((envelope.request_id, envelope.response));
                }
                Ok(Async::Ready(None)) | Ok(Async::NotReady) => break,
                Err(_) => unreachable!(),
            }
        }

        if insertions.len() > 0 || updates.len() > 0 || removals.len() > 0 || responses.len() > 0 {
            let message = serialize(&MessageToClient::Update {
                insertions,
                updates,
                removals,
                responses,
            }).unwrap();
            Ok(Async::Ready(Some(message)))
        } else {
            self.pending_task = Some(task::current());
            Ok(Async::NotReady)
        }
    }
}

impl Stream for ConnectionToClient {
    type Item = Vec<u8>;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.poll_incoming() {
            Ok(true) => {}
            Ok(false) => return Ok(Async::Ready(None)),
            Err(error) => {
                let description = format!("{}", error);
                let message = serialize(&MessageToClient::Err(description)).unwrap();
                return Ok(Async::Ready(Some(message)));
            }
        }

        self.poll_outgoing()
    }
}

impl<T> RawBytesService for T
where
    T: Service,
{
    fn state(&self, connection: &mut ConnectionToClient) -> Vec<u8> {
        serialize(&T::state(self, connection)).unwrap()
    }

    fn poll_update(&mut self, connection: &mut ConnectionToClient) -> Async<Option<Vec<u8>>> {
        T::poll_update(self, connection)
            .map(|option| option.map(|update| serialize(&update).unwrap()))
    }

    fn request(
        &mut self,
        request: Vec<u8>,
        connection: &mut ConnectionToClient,
    ) -> Option<Box<Future<Item = Vec<u8>, Error = Vec<u8>>>> {
        T::request(self, deserialize(&request).unwrap(), connection).map(|future| {
            Box::new(
                future
                    .map(|item| serialize(&item).unwrap())
                    .map_err(|err| serialize(&err).unwrap()),
            ) as Box<Future<Item = Vec<u8>, Error = Vec<u8>>>
        })
    }
}
