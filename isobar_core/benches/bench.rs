extern crate isobar_core;
#[macro_use]
extern crate criterion;

use criterion::Criterion;
use std::cell::RefCell;
use std::rc::Rc;
use isobar_core::buffer::{Buffer, Point};
use isobar_core::buffer_view::BufferView;

fn add_selection(c: &mut Criterion) {
    c.bench_function("add_selection_below", |b| {
        b.iter_with_setup(
            || {
                let mut buffer_view = create_buffer_view(100);
                for i in 0..100 {
                    buffer_view.add_selection(Point::new(i, 0), Point::new(i, 0));
                }
                buffer_view
            },
            |mut buffer_view| buffer_view.add_selection_below(),
        )
    });
    c.bench_function("add_selection_above", |b| {
        b.iter_with_setup(
            || {
                let mut buffer_view = create_buffer_view(100);
                for i in 0..100 {
                    buffer_view.add_selection(Point::new(i, 0), Point::new(i, 0));
                }
                buffer_view
            },
            |mut buffer_view| buffer_view.add_selection_above(),
        )
    });
}

fn edit(c: &mut Criterion) {
    c.bench_function("edit", |b| {
        b.iter_with_setup(
            || {
                let mut buffer_view = create_buffer_view(50);
                for i in 0..50 {
                    buffer_view.add_selection(Point::new(i, 0), Point::new(i, 0));
                }
                buffer_view
            },
            |mut buffer_view| {
                buffer_view.edit("a");
                buffer_view.edit("b");
                buffer_view.edit("c");
            },
        )
    });
}

fn create_buffer_view(lines: usize) -> BufferView {
    let mut buffer = Buffer::new(0);
    for i in 0..lines {
        let len = buffer.len();
        buffer.edit(
            &[len..len],
            format!("Lorem ipsum dolor sit amet {}\n", i).as_str(),
        );
    }
    BufferView::new(Rc::new(RefCell::new(buffer)), 0, None)
}

criterion_group!(benches, edit, add_selection);
criterion_main!(benches);
