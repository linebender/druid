// Copyright 2018 The xi-editor Authors.
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

//! A textbox widget.

use std::time::{Duration, Instant};

use crate::{
    Application, BoxConstraints, Cursor, Env, Event, EventCtx, HotKey, KeyCode, LayoutCtx,
    LifeCycle, LifeCycleCtx, PaintCtx, Selector, SysMods, TimerToken, UpdateCtx, Widget,
};

use crate::kurbo::{Affine, Line, Point, RoundedRect, Size, Vec2};
use crate::piet::{
    FontBuilder, PietText, PietTextLayout, RenderContext, Text, TextLayout, TextLayoutBuilder,
};
use crate::theme;

use crate::text::{BasicTextInput, EditAction, EditableText, MouseAction, TextInput};

use crate::text::{
    config::DEFAULT_CONFIG,
    edit_types::{BufferEvent, EventDomain},
    editor::Editor,
    selection::SelRegion,
    simple_selection::SimpleSelection,
    view::View,
    Movement, Selection,
};

const BORDER_WIDTH: f64 = 1.;
const PADDING_TOP: f64 = 5.;
const PADDING_LEFT: f64 = 4.;

// we send ourselves this when we want to reset blink, which must be done in event.
const RESET_BLINK: Selector = Selector::new("druid-builtin.reset-textbox-blink");

/// A widget that allows user text input.
#[derive(Debug, Clone)]
pub struct TextBox {
    width: f64,
    hscroll_offset: f64,
    selection: SimpleSelection,
    cursor_timer: TimerToken,
    cursor_on: bool,
    placeholder: String,
}

impl TextBox {
    /// Create a new TextBox widget
    pub fn new() -> TextBox {
        Self {
            width: 0.0,
            hscroll_offset: 0.,
            selection: SimpleSelection::caret(0),
            cursor_timer: TimerToken::INVALID,
            cursor_on: false,
            placeholder: String::new(),
        }
    }

    /// Builder-style method to set the `TextBox`'s placeholder text.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    #[deprecated(since = "0.5.0", note = "Use TextBox::new instead")]
    #[doc(hidden)]
    pub fn raw() -> TextBox {
        Self::new()
    }

    /// Calculate the PietTextLayout from the given text, font, and font size
    fn get_layout(&self, piet_text: &mut PietText, text: &str, env: &Env) -> PietTextLayout {
        let font_name = env.get(theme::FONT_NAME);
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        // TODO: caching of both the format and the layout
        let font = piet_text
            .new_font_by_name(font_name, font_size)
            .build()
            .unwrap();

        piet_text
            .new_text_layout(&font, &text.to_string())
            .build()
            .unwrap()
    }

    /// Set the selection to be a caret at the given offset, if that's a valid
    /// codepoint boundary.
    fn caret_to(&mut self, text: &mut String, to: usize) {
        match text.cursor(to) {
            Some(_) => self.selection = SimpleSelection::caret(to),
            None => log::error!("You can't move the cursor there."),
        }
    }

    /// Return the active edge of the current selection or cursor.
    // TODO: is this the right name?
    fn cursor(&self) -> usize {
        self.selection.end
    }

    fn do_edit_action(&mut self, edit_action: EditAction, text: &mut String) {
        let selection = self.selection.constrain_to(text);

        let mut editor = Editor::with_text(text.clone());
        let mut view = View::new();
        view.set_selection(
            editor.get_buffer(),
            Selection::new_simple(SelRegion::new(selection.start, selection.end)),
        );

        let action: EventDomain = edit_action.into();
        match action {
            EventDomain::View(evt) => view.do_edit(editor.get_buffer(), evt),
            EventDomain::Buffer(evt) => editor.do_edit(
                &mut view,
                &mut xi_rope::Rope::default(),
                &DEFAULT_CONFIG,
                evt,
            ),
        }

        let sel_regions = view.sel_regions();
        assert_eq!(sel_regions.len(), 1);
        self.selection = SimpleSelection::new(sel_regions[0].start, sel_regions[0].end);

        let result = editor.get_buffer().to_string();
        *text = result;
    }

    /// For a given point, returns the corresponding offset (in bytes) of
    /// the grapheme cluster closest to that point.
    fn offset_for_point(&self, point: Point, layout: &PietTextLayout) -> usize {
        // Translating from screenspace to Piet's text layout representation.
        // We need to account for hscroll_offset state and TextBox's padding.
        let translated_point = Point::new(point.x + self.hscroll_offset - PADDING_LEFT, point.y);
        let hit_test = layout.hit_test_point(translated_point);
        hit_test.metrics.text_position
    }

    /// Given an offset (in bytes) of a valid grapheme cluster, return
    /// the corresponding x coordinate of that grapheme on the screen.
    fn x_for_offset(&self, layout: &PietTextLayout, offset: usize) -> f64 {
        if let Some(position) = layout.hit_test_text_position(offset) {
            position.point.x
        } else {
            //TODO: what is the correct fallback here?
            0.0
        }
    }

    /// Calculate a stateful scroll offset
    fn update_hscroll(&mut self, layout: &PietTextLayout) {
        let cursor_x = self.x_for_offset(layout, self.cursor());
        let overall_text_width = layout.width();

        let padding = PADDING_LEFT * 2.;
        if overall_text_width < self.width {
            // There's no offset if text is smaller than text box
            //
            // [***I*  ]
            // ^
            self.hscroll_offset = 0.;
        } else if cursor_x > self.width + self.hscroll_offset - padding {
            // If cursor goes past right side, bump the offset
            //       ->
            // **[****I]****
            //   ^
            self.hscroll_offset = cursor_x - self.width + padding;
        } else if cursor_x < self.hscroll_offset {
            // If cursor goes past left side, match the offset
            //    <-
            // **[I****]****
            //   ^
            self.hscroll_offset = cursor_x
        }
    }

    fn reset_cursor_blink(&mut self, ctx: &mut EventCtx) {
        self.cursor_on = true;
        let deadline = Instant::now() + Duration::from_millis(500);
        self.cursor_timer = ctx.request_timer(deadline);
    }
}

impl Widget<String> for TextBox {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut String, env: &Env) {
        // Guard against external changes in data?
        self.selection = self.selection.constrain_to(data);

        let mut text_layout = self.get_layout(&mut ctx.text(), &data, env);
        let mut edit_action = None;

        match event {
            Event::MouseDown(mouse) => {
                ctx.request_focus();
                ctx.set_active(true);

                let cursor_offset = self.offset_for_point(mouse.pos, &text_layout);
                edit_action = Some(EditAction::Click(MouseAction {
                    row: 0,
                    column: cursor_offset,
                    mods: mouse.mods,
                }));

                ctx.request_paint();
            }
            Event::MouseMoved(mouse) => {
                ctx.set_cursor(&Cursor::IBeam);
                if ctx.is_active() {
                    let cursor_offset = self.offset_for_point(mouse.pos, &text_layout);
                    edit_action = Some(EditAction::Drag(MouseAction {
                        row: 0,
                        column: cursor_offset,
                        mods: mouse.mods,
                    }));
                    ctx.request_paint();
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.request_paint();
                }
            }
            Event::Timer(id) => {
                if *id == self.cursor_timer {
                    self.cursor_on = !self.cursor_on;
                    ctx.request_paint();
                    let deadline = Instant::now() + Duration::from_millis(500);
                    self.cursor_timer = ctx.request_timer(deadline);
                }
            }
            Event::Command(ref cmd)
                if ctx.has_focus()
                    && (cmd.selector == crate::commands::COPY
                        || cmd.selector == crate::commands::CUT) =>
            {
                if let Some(text) = data.slice(self.selection.range()) {
                    Application::clipboard().put_string(text);
                }
                if !self.selection.is_caret() && cmd.selector == crate::commands::CUT {
                    edit_action = Some(EditAction::Delete(Movement::Right));
                }
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.selector == RESET_BLINK => self.reset_cursor_blink(ctx),
            Event::Paste(ref item) => {
                if let Some(string) = item.get_string() {
                    edit_action = Some(EditAction::Paste(string));
                    ctx.request_paint();
                }
            }
            Event::KeyDown(key_event) => {
                let event_handled = match key_event {
                    // Tab and shift+tab
                    k_e if HotKey::new(None, KeyCode::Tab).matches(k_e) => {
                        ctx.focus_next();
                        true
                    }
                    k_e if HotKey::new(SysMods::Shift, KeyCode::Tab).matches(k_e) => {
                        ctx.focus_prev();
                        true
                    }
                    _ => false,
                };

                if !event_handled {
                    edit_action = BasicTextInput::new().handle_event(key_event);
                }

                ctx.request_paint();
            }
            _ => (),
        }

        if let Some(edit_action) = edit_action {
            let is_select_all = if let EditAction::SelectAll = &edit_action {
                true
            } else {
                false
            };

            self.do_edit_action(edit_action, data);
            self.reset_cursor_blink(ctx);

            if !is_select_all {
                text_layout = self.get_layout(&mut ctx.text(), &data, env);
                self.update_hscroll(&text_layout);
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &String, _env: &Env) {
        match event {
            LifeCycle::WidgetAdded => ctx.register_for_focus(),
            // an open question: should we be able to schedule timers here?
            LifeCycle::FocusChanged(true) => ctx.submit_command(RESET_BLINK, ctx.widget_id()),
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &String, _data: &String, _env: &Env) {
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &String,
        env: &Env,
    ) -> Size {
        let width = env.get(theme::WIDE_WIDGET_WIDTH);
        let height = env.get(theme::BORDERED_WIDGET_HEIGHT);

        let size = bc.constrain((width, height));
        self.width = size.width;
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &String, env: &Env) {
        // Guard against changes in data following `event`
        let content = if data.is_empty() {
            &self.placeholder
        } else {
            data
        };

        self.selection = self.selection.constrain_to(content);

        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        let height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let background_color = env.get(theme::BACKGROUND_LIGHT);
        let selection_color = env.get(theme::SELECTION_COLOR);
        let text_color = env.get(theme::LABEL_COLOR);
        let placeholder_color = env.get(theme::PLACEHOLDER_COLOR);
        let cursor_color = env.get(theme::CURSOR_COLOR);

        let has_focus = ctx.has_focus();

        let border_color = if has_focus {
            env.get(theme::PRIMARY_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        };

        // Paint the background
        let clip_rect = RoundedRect::from_origin_size(
            Point::ORIGIN,
            Size::new(self.width - BORDER_WIDTH, height).to_vec2(),
            env.get(theme::TEXTBOX_BORDER_RADIUS),
        );

        ctx.fill(clip_rect, &background_color);

        // Render text, selection, and cursor inside a clip
        ctx.with_save(|rc| {
            rc.clip(clip_rect);

            // Calculate layout
            let text_layout = self.get_layout(rc.text(), &content, env);

            // Shift everything inside the clip by the hscroll_offset
            rc.transform(Affine::translate((-self.hscroll_offset, 0.)));

            // Draw selection rect
            if !self.selection.is_caret() {
                let (left, right) = (self.selection.min(), self.selection.max());
                let left_offset = self.x_for_offset(&text_layout, left);
                let right_offset = self.x_for_offset(&text_layout, right);

                let selection_width = right_offset - left_offset;

                let selection_pos = Point::new(left_offset + PADDING_LEFT - 1., PADDING_TOP - 2.);

                let selection_rect = RoundedRect::from_origin_size(
                    selection_pos,
                    Size::new(selection_width + 2., font_size + 4.).to_vec2(),
                    1.,
                );
                rc.fill(selection_rect, &selection_color);
            }

            // Layout, measure, and draw text
            let text_height = font_size * 0.8;
            let text_pos = Point::new(0.0 + PADDING_LEFT, text_height + PADDING_TOP);
            let color = if data.is_empty() {
                &placeholder_color
            } else {
                &text_color
            };

            rc.draw_text(&text_layout, text_pos, color);

            // Paint the cursor if focused and there's no selection
            if has_focus && self.cursor_on && self.selection.is_caret() {
                let cursor_x = self.x_for_offset(&text_layout, self.cursor());
                let xy = text_pos + Vec2::new(cursor_x, 2. - font_size);
                let x2y2 = xy + Vec2::new(0., font_size + 2.);
                let line = Line::new(xy, x2y2);

                rc.stroke(line, &cursor_color, 1.);
            }
        });

        // Paint the border
        ctx.stroke(clip_rect, &border_color, BORDER_WIDTH);
    }
}

impl Default for TextBox {
    fn default() -> Self {
        TextBox::new()
    }
}
