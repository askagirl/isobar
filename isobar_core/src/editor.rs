use std::rc::Rc;
use std::cell::RefCell;
use std::cmp::{self, Ordering};
use futures::future::Executor;
use futures::{Future, Stream};
use notify_cell::NotifyCell;
use buffer::{Buffer, Version, Point, Anchor};

pub struct Editor {
    buffer: Rc<RefCell<Buffer>>,
    pub version: Rc<NotifyCell<Version>>,
    dropped: NotifyCell<bool>,
    selections: Vec<Selection>
}

struct Selection {
    start: Anchor,
    end: Anchor,
    reversed: bool,
    goal_column: Option<u32>
}

pub mod render {
    use super::Point;

    pub struct Params {
        pub scroll_top: f64,
        pub height: f64,
        pub line_height: f64
    }

    pub struct Frame {
        pub first_visible_row: u32,
        pub lines: Vec<Vec<u16>>,
        pub selections: Vec<Selection>
    }

    #[derive(Debug, Eq, PartialEq)]
    pub struct Selection {
        pub start: Point,
        pub end: Point,
        pub reversed: bool
    }
}

impl Editor {
    pub fn new(buffer: Rc<RefCell<Buffer>>) -> Self {
        let version;
        let selections;

        {
            let buffer = buffer.borrow();
            version = buffer.version.get().unwrap();
            selections = vec![Selection {
                start: buffer.anchor_before_offset(0).unwrap(),
                end: buffer.anchor_before_offset(0).unwrap(),
                reversed: false,
                goal_column: None
            }];
        }

        Self {
            version: Rc::new(NotifyCell::new(version)),
            buffer,
            selections,
            dropped: NotifyCell::new(false),
        }
    }

    pub fn run<E>(&self, executor: &E)
    where
        E: Executor<Box<Future<Item = (), Error = ()>>>,
    {
        let version_cell = self.version.clone();
        let buffer_observation = self.buffer.borrow().version.observe().for_each(
            move |buffer_version| {
                version_cell.set(buffer_version);
                Ok(())
            },
        );
        let drop_observation = self.dropped.observe().into_future();
        executor.execute(Box::new(
            buffer_observation
                .select2(drop_observation)
                .then(|_| Ok(())),
        )).unwrap();
    }

    pub fn render(&self, params: render::Params) -> render::Frame {
        let buffer = self.buffer.borrow();
        let mut lines = Vec::new();
        let mut cur_line = Vec::new();
        let scroll_bottom = params.scroll_top + params.height;
        let start_row = (params.scroll_top / params.line_height).floor() as u32;
        let end_row = (scroll_bottom / params.line_height).ceil() as u32;

        let mut cur_row = start_row;
        for c in buffer.iter_starting_at_row(start_row) {
            if c == (b'\n' as u16) {
                lines.push(cur_line);
                cur_line = Vec::new();
                cur_row += 1;
                if cur_row >= end_row {
                    break;
                }
            } else {
                cur_line.push(c);
            }
        }
        if cur_row < end_row {
            lines.push(cur_line);
        }

        render::Frame {
            first_visible_row: start_row,
            lines,
            selections: self.selections.iter().map(|selection| selection.render(&buffer)).collect()
        }
    }

    pub fn add_selection(&mut self, start: Point, end: Point) {
        debug_assert!(start <= end); // TODO: Reverse selection if end < start

        {
            let buffer = self.buffer.borrow();

            // TODO: Clip points or return a result.
            let start_anchor = buffer.anchor_before_point(start).unwrap();
            let end_anchor = buffer.anchor_before_point(end).unwrap();
            let index = match self.selections.binary_search_by(|probe| buffer.cmp_anchors(&probe.start, &start_anchor).unwrap()) {
                Ok(index) => index,
                Err(index) => index
            };
            self.selections.insert(index, Selection {
                start: start_anchor,
                end: end_anchor,
                reversed: false,
                goal_column: None
            });
        }

        self.merge_selections();
    }

    fn merge_selections(&mut self) {
        let buffer = self.buffer.borrow();
        let mut i = 1;
        while i < self.selections.len() {
            if buffer.cmp_anchors(&self.selections[i - 1].end, &self.selections[i].start).unwrap() >= Ordering::Equal {
                let removed = self.selections.remove(i);
                if buffer.cmp_anchors(&removed.end, &self.selections[i - 1].end).unwrap() > Ordering::Equal {
                    self.selections[i - 1].end = removed.end;
                }
            } else {
                i += 1;
            }
        }
    }

    pub fn move_right(&mut self) {
        let buffer = self.buffer.borrow();
        let max_offset = buffer.len();
        self.selections = self.selections.iter().map(|selection| {
            let new_offset = cmp::min(
                max_offset,
                buffer.offset_for_anchor(selection.head()).unwrap() + 1
            );
            let new_anchor = buffer.anchor_before_offset(new_offset).unwrap();
            Selection {
                start: new_anchor.clone(),
                end: new_anchor,
                reversed: false,
                goal_column: None
            }
        }).collect();
    }

    pub fn move_left(&mut self) {
        let buffer = self.buffer.borrow();
        self.selections = self.selections.iter().map(|selection| {
            let new_offset = buffer.offset_for_anchor(selection.head()).unwrap().saturating_sub(1);
            let new_anchor = buffer.anchor_before_offset(new_offset).unwrap();
            Selection {
                start: new_anchor.clone(),
                end: new_anchor,
                reversed: false,
                goal_column: None
            }
        }).collect();
    }

    pub fn move_up(&mut self) {
        let buffer = self.buffer.borrow();
        self.selections = self.selections.iter().map(|selection| {
            let goal_column;
            let mut new_point = buffer.point_for_anchor(selection.head()).unwrap();
            if new_point.row > 0 {
                new_point.row -= 1;
                new_point.column = selection.goal_column.unwrap_or(new_point.column);

                let row_len = buffer.len_for_row(new_point.row).unwrap();
                if new_point.column > row_len {
                    goal_column = Some(new_point.column);
                    new_point.column = row_len;
                } else {
                    goal_column = None;
                }
            } else {
                goal_column = selection.goal_column.or(Some(new_point.column));
                new_point = Point::new(0, 0);
            }

            let new_anchor = buffer.anchor_before_point(new_point).unwrap();
            Selection {
                start: new_anchor.clone(),
                end: new_anchor,
                reversed: false,
                goal_column
            }
        }).collect();
    }

    pub fn move_down(&mut self) {
        let buffer = self.buffer.borrow();
        self.selections = self.selections.iter().map(|selection| {
            let max_point = buffer.max_point();

            let goal_column;
            let mut new_point = buffer.point_for_anchor(selection.head()).unwrap();
            if new_point.row < max_point.row {
                new_point.row += 1;
                new_point.column = selection.goal_column.unwrap_or(new_point.column);

                let row_len = buffer.len_for_row(new_point.row).unwrap();
                if new_point.column > row_len {
                    goal_column = Some(new_point.column);
                    new_point.column = row_len;
                } else {
                    goal_column = None;
                }
            } else {
                goal_column = selection.goal_column.or(Some(new_point.column));
                new_point = max_point;
            }

            let new_anchor = buffer.anchor_before_point(new_point).unwrap();
            Selection {
                start: new_anchor.clone(),
                end: new_anchor,
                reversed: false,
                goal_column
            }
        }).collect();
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.dropped.set(true);
    }
}

impl Selection {
    fn head(&self) -> &Anchor {
        if self.reversed {
            &self.start
        } else {
            &self.end
        }
    }

    fn render(&self, buffer: &Buffer) -> render::Selection {
        render::Selection {
            start: buffer.point_for_anchor(&self.start).unwrap(),
            end: buffer.point_for_anchor(&self.end).unwrap(),
            reversed: self.reversed
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate tokio_core;

    use super::*;
    use self::tokio_core::reactor::Core;
    use futures::future;

    #[test]
    fn test_version_updates() {
        let mut event_loop = Core::new().unwrap();
        let buffer = Rc::new(RefCell::new(Buffer::new(1)));
        let editor = Editor::new(buffer.clone());
        editor.run(&event_loop);
        buffer.borrow_mut().splice(0..0, "test");
        event_loop.run(editor.version.observe().take(1).into_future());
    }

    #[test]
    fn test_cursor_movement() {
        let mut editor = Editor::new(Rc::new(RefCell::new(Buffer::new(1))));
        editor.buffer.borrow_mut().splice(0..0, "abc\n\ndef");
        assert_eq!(render_selections(&editor), vec![empty_selection(0, 0)]);

        editor.move_right();
        assert_eq!(render_selections(&editor), vec![empty_selection(0, 1)]);

        // Wraps across lines moving right
        for _ in 0..3 { editor.move_right(); }
        assert_eq!(render_selections(&editor), vec![empty_selection(1, 0)]);

        // Stops at end
        for _ in 0..4 { editor.move_right(); }
        assert_eq!(render_selections(&editor), vec![empty_selection(2, 3)]);

        // Wraps across lines moving left
        for _ in 0..4 { editor.move_left(); }
        assert_eq!(render_selections(&editor), vec![empty_selection(1, 0)]);

        // Stops at start
        for _ in 0..4 { editor.move_left(); }
        assert_eq!(render_selections(&editor), vec![empty_selection(0, 0)]);

        // Moves down and up at column 0
        editor.move_down();
        assert_eq!(render_selections(&editor), vec![empty_selection(1, 0)]);
        editor.move_up();
        assert_eq!(render_selections(&editor), vec![empty_selection(0, 0)]);

        // Maintains a goal column when moving down
        // This means we'll jump to the column we started with even after crossing a shorter line
        editor.move_right();
        editor.move_right();
        editor.move_down();
        assert_eq!(render_selections(&editor), vec![empty_selection(1, 0)]);
        editor.move_down();
        assert_eq!(render_selections(&editor), vec![empty_selection(2, 2)]);

        // Jumps to end when moving down on the last line.
        editor.move_down();
        assert_eq!(render_selections(&editor), vec![empty_selection(2, 3)]);

        // Stops at end
        editor.move_down();
        assert_eq!(render_selections(&editor), vec![empty_selection(2, 3)]);

        // Resets the goal column when moving horizontally
        editor.move_left();
        editor.move_left();
        editor.move_up();
        assert_eq!(render_selections(&editor), vec![empty_selection(1, 0)]);
        editor.move_up();
        assert_eq!(render_selections(&editor), vec![empty_selection(0, 1)]);

        // Jumps to start when moving up on the first line
        editor.move_up();
        assert_eq!(render_selections(&editor), vec![empty_selection(0, 0)]);

        // Preserves goal column after jumping to start/end
        editor.move_down();
        editor.move_down();
        assert_eq!(render_selections(&editor), vec![empty_selection(2, 1)]);
        editor.move_down();
        assert_eq!(render_selections(&editor), vec![empty_selection(2, 3)]);
        editor.move_up();
        editor.move_up();
        assert_eq!(render_selections(&editor), vec![empty_selection(0, 1)]);
    }

    #[test]
    fn test_add_selection() {
        let mut editor = Editor::new(Rc::new(RefCell::new(Buffer::new(1))));
        editor.buffer.borrow_mut().splice(0..0, "abcd\nefgh\nijkl\nmnop");
        assert_eq!(render_selections(&editor), vec![empty_selection(0 ,0)]);

        // Adding non-overlapping selections
        editor.move_right();
        editor.move_right();
        editor.add_selection(Point::new(0, 0), Point::new(0, 1));
        editor.add_selection(Point::new(2, 2), Point::new(2, 3));
        editor.add_selection(Point::new(0, 3), Point::new(1, 2));
        assert_eq!(
            render_selections(&editor),
            vec![
                selection((0, 0), (0, 1)),
                selection((0, 2), (0, 2)),
                selection((0, 3), (1, 2)),
                selection((2, 2), (2, 3))
            ]
        );

        // Adding a selection that starts at the start of an existing selection
        editor.add_selection(Point::new(0, 3), Point::new(1, 0));
        editor.add_selection(Point::new(0, 3), Point::new(1, 3));
        editor.add_selection(Point::new(0, 3), Point::new(1, 2));

        assert_eq!(
            render_selections(&editor),
            vec![
                selection((0, 0), (0, 1)),
                selection((0, 2), (0, 2)),
                selection((0, 3), (1, 3)),
                selection((2, 2), (2, 3))
            ]
        );

        // Adding a selection that starts or ends inside an existing selection
        editor.add_selection(Point::new(0, 1), Point::new(0, 2));
        editor.add_selection(Point::new(1, 2), Point::new(1, 4));
        editor.add_selection(Point::new(2, 1), Point::new(2, 2));
        assert_eq!(
            render_selections(&editor),
            vec![
                selection((0, 0), (0, 2)),
                selection((0, 3), (1, 4)),
                selection((2, 1), (2, 3))
            ]
        );
    };

    fn render_selections(editor: &Editor) -> Vec<render::Selection> {
        editor.selections.iter().map(|s| s.render(&editor.buffer.borrow())).collect()
    }

    fn empty_selection(row: u32, column: u32) -> render::Selection {
        render::Selection {
            start: Point::new(row, column),
            end: Point::new(row, column),
            reversed: false
        }
    }

    fn selection(start: (u32, u32), end: (u32, u32)) -> render::Selection {
        render::Selection {
            start: Point::new(start.0, start.1),
            end: Point::new(end.0, end.1),
            reversed: false
        }
    }
}
