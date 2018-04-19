use serde_json;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};
use futures::{Async, Poll, Stream};
use futures::task::{self, Task};

pub type ViewId = usize;
pub type ViewUpdateStream = Box<Stream<Item = (), Error = ()>>;

pub trait View {
    fn component_name(&self) -> &'static str;
    fn did_mount(&mut self, _handle: WindowHandle) {}
    fn render(&self) -> serde_json::Value;
    fn updates(&self) -> ViewUpdateStream;
    fn dispatch_action(&mut self, serde_json::Value);
}

pub struct Window(Rc<RefCell<Inner>>, Option<ViewHandle>);
pub struct WindowUpdateStream {
    counter: usize,
    polled_once: bool,
    inner: Weak<RefCell<Inner>>,
}

pub struct Inner {
    next_view_id: ViewId,
    views: HashMap<ViewId, (Rc<RefCell<View>>, RefCell<ViewUpdateStream>)>,
    inserted: HashSet<ViewId>,
    removed: HashSet<ViewId>,
    height: f64,
    update_stream_counter: usize,
    update_stream_task: Option<Task>,
}

pub struct WindowHandle(Weak<RefCell<Inner>>);

pub struct ViewHandle {
    pub view_id: ViewId,
    inner: Weak<RefCell<Inner>>,
}

#[derive(Serialize, Debug)]
pub struct WindowUpdate {
    updated: Vec<ViewUpdate>,
    removed: Vec<ViewId>,
}

#[derive(Serialize, Debug)]
pub struct ViewUpdate {
    component_name: &'static str,
    view_id: ViewId,
    props: serde_json::Value,
}

impl Window {
    pub fn new(height: f64) -> Self {
        Window(
            Rc::new(RefCell::new(Inner {
                next_view_id: 0,
                views: HashMap::new(),
                inserted: HashSet::new(),
                removed: HashSet::new(),
                height: height,
                update_stream_counter: 0,
                update_stream_task: None,
            })),
            None,
        )
    }

    pub fn dispatch_action(&self, view_id: ViewId, action: serde_json::Value) {
        let view = self.0.borrow().get_view(view_id);
        view.map(|view| view.borrow_mut().dispatch_action(action));
    }

    pub fn updates(&mut self) -> WindowUpdateStream {
        let mut inner = self.0.borrow_mut();
        inner.update_stream_counter += 1;
        WindowUpdateStream {
            counter: inner.update_stream_counter,
            polled_once: false,
            inner: Rc::downgrade(&self.0),
        }
    }

    pub fn set_height(&mut self, height: f64) {
        let mut inner = self.0.borrow_mut();
        inner.height = height;
    }

    pub fn set_root_view(&mut self, root_view: ViewHandle) {
        self.1 = Some(root_view);
    }

    pub fn handle(&mut self) -> WindowHandle {
        WindowHandle(Rc::downgrade(&self.0))
    }
}

impl Stream for WindowUpdateStream {
    type Item = WindowUpdate;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let inner_ref = match self.inner.upgrade() {
            None => return Ok(Async::Ready(None)),
            Some(inner) => inner,
        };

        let mut window_update;
        {
            let inner = inner_ref.borrow();

            if self.counter < inner.update_stream_counter {
                return Ok(Async::Ready(None));
            }

            if self.polled_once {
                window_update = WindowUpdate {
                    updated: Vec::new(),
                    removed: inner.removed.iter().cloned().collect(),
                };

                for id in inner.inserted.iter() {
                    if !inner.removed.contains(&id) {
                        let view = inner.get_view(*id).unwrap();
                        let view = view.borrow();
                        window_update.updated.push(ViewUpdate {
                            view_id: *id,
                            component_name: view.component_name(),
                            props: view.render(),
                        });
                    }
                }

                for (id, &(ref view, ref updates)) in inner.views.iter() {
                    let result = updates.borrow_mut().poll();
                    if !inner.inserted.contains(&id) {
                        if let Ok(Async::Ready(Some(()))) = result {
                            let view = view.borrow();
                            window_update.updated.push(ViewUpdate {
                                view_id: *id,
                                component_name: view.component_name(),
                                props: view.render(),
                            });
                        }
                    }
                }
            } else {
                window_update = WindowUpdate {
                    updated: Vec::new(),
                    removed: Vec::new(),
                };

                for (id, &(ref view, ref updates)) in inner.views.iter() {
                    let _ = updates.borrow_mut().poll();
                    let view = view.borrow();
                    window_update.updated.push(ViewUpdate {
                        view_id: *id,
                        component_name: view.component_name(),
                        props: view.render(),
                    });
                }

                self.polled_once = true;
            }
        }

        let mut inner = inner_ref.borrow_mut();
        inner.inserted.clear();
        inner.removed.clear();

        if window_update.removed.is_empty() && window_update.updated.is_empty() {
            inner.update_stream_task = Some(task::current());
            Ok(Async::NotReady)
        } else {
            Ok(Async::Ready(Some(window_update)))
        }
    }
}

impl Inner {
    fn get_view(&self, id: ViewId) -> Option<Rc<RefCell<View>>> {
        self.views.get(&id).map(|&(ref view, _)| view.clone())
    }
}

impl WindowHandle {
    pub fn height(&self) -> f64 {
        let inner = self.0.upgrade().unwrap();
        let inner = inner.borrow();
        inner.height
    }

    pub fn add_view<T: 'static + View>(&self, mut view: T) -> ViewHandle {
        let view_id = {
            let inner = self.0.upgrade().unwrap();
            let mut inner = inner.borrow_mut();
            inner.next_view_id += 1;
            inner.next_view_id - 1
        };

        view.did_mount(WindowHandle(self.0.clone()));
        let updates = view.updates();

        let inner = self.0.upgrade().unwrap();
        let mut inner = inner.borrow_mut();
        inner.views.insert(
            view_id,
            (Rc::new(RefCell::new(view)), RefCell::new(updates)),
        );
        inner.inserted.insert(view_id);
        inner.update_stream_task.take().map(|task| task.notify());
        ViewHandle {
            view_id,
            inner: self.0.clone(),
        }
    }
}

impl Drop for ViewHandle {
    fn drop(&mut self) {
        let inner = self.inner.upgrade();
        if let Some(inner) = inner {
            let mut inner = inner.borrow_mut();
            inner.views.remove(&self.view_id);
            inner.removed.insert(self.view_id);
            inner.update_stream_task.take().map(|task| task.notify());
        }
    }
}
