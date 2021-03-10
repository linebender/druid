// Copyright 2020 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! This example shows a how a single-line text field might be implemented for druid-shell.
//! Beyond the omission of multiple lines and text wrapping, it also is missing many motions
//! (like "move to previous word") and bidirectional text support.

use std::any::Any;
use std::borrow::Cow;
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use unicode_segmentation::GraphemeCursor;

use druid_shell::kurbo::Size;
use druid_shell::piet::{
    Color, FontFamily, HitTestPoint, PietText, PietTextLayout, RenderContext, Text, TextLayout,
    TextLayoutBuilder,
};

use druid_shell::{
    keyboard_types::Key, text, text::Action, text::Event, text::InputHandler, text::Selection,
    text::VerticalMovement, Application, KeyEvent, Region, TextFieldToken, WinHandler,
    WindowBuilder, WindowHandle,
};

use druid_shell::kurbo::{Point, Rect};

const BG_COLOR: Color = Color::rgb8(0xff, 0xff, 0xff);
const COMPOSITION_BG_COLOR: Color = Color::rgb8(0xff, 0xd8, 0x6e);
const SELECTION_BG_COLOR: Color = Color::rgb8(0x87, 0xc5, 0xff);
const CARET_COLOR: Color = Color::rgb8(0x00, 0x82, 0xfc);
const FONT: FontFamily = FontFamily::SANS_SERIF;
const FONT_SIZE: f64 = 16.0;

#[derive(Default)]
struct AppState {
    size: Size,
    handle: WindowHandle,
    document: Rc<RefCell<DocumentState>>,
    text_input_token: Option<TextFieldToken>,
}

#[derive(Default)]
struct DocumentState {
    text: String,
    selection: Selection,
    composition: Option<Range<usize>>,
    text_engine: Option<PietText>,
    layout: Option<PietTextLayout>,
}

impl DocumentState {
    fn refresh_layout(&mut self) {
        let text_engine = self.text_engine.as_mut().unwrap();
        self.layout = Some(
            text_engine
                .new_text_layout(self.text.clone())
                .font(FONT, FONT_SIZE)
                .build()
                .unwrap(),
        );
    }
}

impl WinHandler for AppState {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
        let token = self.handle.add_text_field();
        self.handle.set_focused_text_field(Some(token));
        self.text_input_token = Some(token);
        let mut doc = self.document.borrow_mut();
        doc.text_engine = Some(handle.text());
        doc.refresh_layout();
    }

    fn prepare_paint(&mut self) {
        self.handle.invalidate();
    }

    fn paint(&mut self, piet: &mut piet_common::Piet, _: &Region) {
        let rect = self.size.to_rect();
        piet.fill(rect, &BG_COLOR);
        let doc = self.document.borrow();
        let layout = doc.layout.as_ref().unwrap();
        // TODO(lord): rects for range on layout
        if let Some(composition_range) = doc.composition.as_ref() {
            for rect in layout.rects_for_range(composition_range.clone()) {
                piet.fill(rect, &COMPOSITION_BG_COLOR);
            }
        }
        if !doc.selection.is_caret() {
            for rect in layout.rects_for_range(doc.selection.to_range()) {
                piet.fill(rect, &SELECTION_BG_COLOR);
            }
        }
        piet.draw_text(layout, (0.0, 0.0));

        // draw caret
        let caret_x = layout.hit_test_text_position(doc.selection.active).point.x;
        piet.fill(
            Rect::new(caret_x - 1.0, 0.0, caret_x + 1.0, FONT_SIZE),
            &CARET_COLOR,
        );
    }

    fn command(&mut self, id: u32) {
        match id {
            0x100 => {
                self.handle.close();
                Application::global().quit()
            }
            _ => println!("unexpected id {}", id),
        }
    }

    fn key_down(&mut self, event: KeyEvent) -> bool {
        if event.key == Key::Character("c".to_string()) {
            // custom hotkey for pressing "c"
            println!("user pressed c! wow! setting selection to 0");

            // update internal selection state
            self.document.borrow_mut().selection = Selection::new_caret(0);

            // notify the OS that we've updated the selection
            self.handle
                .update_text_field(self.text_input_token.unwrap(), Event::SelectionChanged);

            // repaint window
            self.handle.request_anim_frame();

            // return true prevents the keypress event from being handled as text input
            return true;
        }
        false
    }

    fn acquire_input_lock(
        &mut self,
        _token: TextFieldToken,
        _mutable: bool,
    ) -> Box<dyn InputHandler> {
        Box::new(AppInputHandler {
            state: self.document.clone(),
            window_size: self.size,
            window_handle: self.handle.clone(),
        })
    }

    fn release_input_lock(&mut self, _token: TextFieldToken) {
        // no action required; this example is simple enough that this
        // state is not actually shared.
    }

    fn size(&mut self, size: Size) {
        self.size = size;
    }

    fn request_close(&mut self) {
        self.handle.close();
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

struct AppInputHandler {
    state: Rc<RefCell<DocumentState>>,
    window_size: Size,
    window_handle: WindowHandle,
}

impl InputHandler for AppInputHandler {
    fn selection(&self) -> Selection {
        self.state.borrow().selection.clone()
    }
    fn composition_range(&self) -> Option<Range<usize>> {
        self.state.borrow().composition.clone()
    }
    fn set_selection(&mut self, range: Selection) {
        self.state.borrow_mut().selection = range;
        self.window_handle.request_anim_frame();
    }
    fn set_composition_range(&mut self, range: Option<Range<usize>>) {
        self.state.borrow_mut().composition = range;
        self.window_handle.request_anim_frame();
    }
    fn replace_range(&mut self, range: Range<usize>, text: &str) {
        let mut doc = self.state.borrow_mut();
        doc.text.replace_range(range.clone(), text);
        if doc.selection.anchor < range.start && doc.selection.active < range.start {
            // no need to update selection
        } else if doc.selection.anchor > range.end && doc.selection.active > range.end {
            doc.selection.anchor -= range.len();
            doc.selection.active -= range.len();
            doc.selection.anchor += text.len();
            doc.selection.active += text.len();
        } else {
            doc.selection.anchor = range.start + text.len();
            doc.selection.active = range.start + text.len();
        }
        doc.refresh_layout();
        doc.composition = None;
        self.window_handle.request_anim_frame();
    }
    fn slice(&self, range: Range<usize>) -> Cow<str> {
        self.state.borrow().text[range].to_string().into()
    }
    fn is_char_boundary(&self, i: usize) -> bool {
        self.state.borrow().text.is_char_boundary(i)
    }
    fn len(&self) -> usize {
        self.state.borrow().text.len()
    }
    fn hit_test_point(&self, point: Point) -> HitTestPoint {
        self.state
            .borrow()
            .layout
            .as_ref()
            .unwrap()
            .hit_test_point(point)
    }
    fn bounding_box(&self) -> Option<Rect> {
        Some(Rect::new(
            0.0,
            0.0,
            self.window_size.width,
            self.window_size.height,
        ))
    }
    fn slice_bounding_box(&self, range: Range<usize>) -> Option<Rect> {
        let doc = self.state.borrow();
        let layout = doc.layout.as_ref().unwrap();
        let range_start_x = layout.hit_test_text_position(range.start).point.x;
        let range_end_x = layout.hit_test_text_position(range.end).point.x;
        Some(Rect::new(range_start_x, 0.0, range_end_x, FONT_SIZE))
    }
    fn line_range(&self, _char_index: usize, _affinity: text::Affinity) -> Range<usize> {
        // we don't have multiple lines, so no matter the input, output is the whole document
        0..self.state.borrow().text.len()
    }

    fn handle_action(&mut self, action: Action) {
        let handled = apply_default_behavior(self, action);
        println!("action: {:?} handled: {:?}", action, handled);
    }
}

fn apply_default_behavior(handler: &mut AppInputHandler, action: Action) -> bool {
    let is_caret = handler.selection().is_caret();
    match action {
        Action::Move(movement) => {
            let selection = handler.selection();
            let index = if movement_goes_downstream(movement) {
                selection.max()
            } else {
                selection.min()
            };
            let updated_index = if let (false, text::Movement::Grapheme(_)) = (is_caret, movement) {
                // handle special cases of pressing left/right when the selection is not a caret
                index
            } else {
                match apply_movement(handler, movement, index) {
                    Some(v) => v,
                    None => return false,
                }
            };
            handler.set_selection(Selection::new_caret(updated_index));
        }
        Action::MoveSelecting(movement) => {
            let mut selection = handler.selection();
            selection.active = match apply_movement(handler, movement, selection.active) {
                Some(v) => v,
                None => return false,
            };
            handler.set_selection(selection);
        }
        Action::SelectAll => {
            let len = handler.len();
            let selection = Selection {
                anchor: 0,
                active: len,
            };
            handler.set_selection(selection);
        }
        Action::Delete(_) if !is_caret => {
            // movement is ignored for non-caret selections
            let selection = handler.selection();
            handler.replace_range(selection.to_range(), "");
        }
        Action::Delete(movement) => {
            let mut selection = handler.selection();
            selection.active = match apply_movement(handler, movement, selection.active) {
                Some(v) => v,
                None => return false,
            };
            handler.replace_range(selection.to_range(), "");
        }
        _ => return false,
    }
    true
}

fn movement_goes_downstream(movement: text::Movement) -> bool {
    match movement {
        text::Movement::Grapheme(dir) => direction_goes_downstream(dir),
        text::Movement::Word(dir) => direction_goes_downstream(dir),
        text::Movement::Line(dir) => direction_goes_downstream(dir),
        text::Movement::ParagraphEnd => true,
        text::Movement::Vertical(VerticalMovement::LineDown) => true,
        text::Movement::Vertical(VerticalMovement::PageDown) => true,
        text::Movement::Vertical(VerticalMovement::DocumentEnd) => true,
        _ => false,
    }
}

fn direction_goes_downstream(direction: text::Direction) -> bool {
    match direction {
        text::Direction::Left => false,
        text::Direction::Right => true,
        text::Direction::Upstream => false,
        text::Direction::Downstream => true,
    }
}

fn apply_movement(
    edit_lock: &mut AppInputHandler,
    movement: text::Movement,
    index: usize,
) -> Option<usize> {
    match movement {
        text::Movement::Grapheme(dir) => {
            let doc_len = edit_lock.len();
            let mut cursor = GraphemeCursor::new(index, doc_len, true);
            let doc = edit_lock.slice(0..doc_len);
            if direction_goes_downstream(dir) {
                cursor.next_boundary(&doc, 0).unwrap()
            } else {
                cursor.prev_boundary(&doc, 0).unwrap()
            }
        }
        _ => None,
    }
}

fn main() {
    let app = Application::new().unwrap();
    let mut builder = WindowBuilder::new(app.clone());
    builder.set_handler(Box::new(AppState::default()));
    builder.set_title("Text editing example");
    let window = builder.build().unwrap();
    window.show();
    app.run(None);
}
