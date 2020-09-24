// Copyright 2020 The xi-editor Authors.
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

//! A type for laying out, drawing, and interacting with text.

use std::ops::Range;

use crate::kurbo::{Line, Point, Rect, Size};
use crate::piet::{
    Color, PietText, PietTextLayout, Text as _, TextAlignment, TextAttribute, TextLayout as _,
    TextLayoutBuilder as _,
};
use crate::{ArcStr, Env, FontDescriptor, KeyOrValue, PaintCtx, RenderContext, UpdateCtx};

/// A component for displaying text on screen.
///
/// This is a type intended to be used by other widgets that display text.
/// It allows for the text itself as well as font and other styling information
/// to be set and modified. It wraps an inner layout object, and handles
/// invalidating and rebuilding it as required.
///
/// This object is not valid until the [`rebuild_if_needed`] method has been
/// called. You should generally do this in your widget's [`layout`] method.
/// Additionally, you should call [`needs_rebuild_after_update`]
/// as part of your widget's [`update`] method; if this returns `true`, you will need
/// to call [`rebuild_if_needed`] again, generally by scheduling another [`layout`]
/// pass.
///
/// [`layout`]: trait.Widget.html#tymethod.layout
/// [`update`]: trait.Widget.html#tymethod.update
/// [`needs_rebuild_after_update`]: #method.needs_rebuild_after_update
/// [`rebuild_if_needed`]: #method.rebuild_if_needed
/// [`Env`]: struct.Env.html
#[derive(Clone)]
pub struct TextLayout {
    text: ArcStr,
    font: KeyOrValue<FontDescriptor>,
    // when set, this will be used to override the size in he font descriptor.
    // This provides an easy way to change only the font size, while still
    // using a `FontDescriptor` in the `Env`.
    text_size_override: Option<KeyOrValue<f64>>,
    text_color: KeyOrValue<Color>,
    layout: Option<PietTextLayout>,
    wrap_width: f64,
    alignment: TextAlignment,
}

impl TextLayout {
    /// Create a new `TextLayout` object.
    ///
    /// You do not provide the actual text at creation time; instead you pass
    /// it in when calling [`rebuild_if_needed`].
    ///
    /// [`rebuild_if_needed`]: #method.rebuild_if_needed
    pub fn new(text: impl Into<ArcStr>) -> Self {
        TextLayout {
            text: text.into(),
            font: crate::theme::UI_FONT.into(),
            text_color: crate::theme::LABEL_COLOR.into(),
            text_size_override: None,
            layout: None,
            wrap_width: f64::INFINITY,
            alignment: Default::default(),
        }
    }

    /// Returns `true` if this layout needs to be rebuilt.
    ///
    /// This happens (for instance) after style attributes are modified.
    ///
    /// This does not account for things like the text changing, handling that
    /// is the responsibility of the user.
    pub fn needs_rebuild(&self) -> bool {
        self.layout.is_none()
    }

    /// Set the text to display.
    pub fn set_text(&mut self, text: impl Into<ArcStr>) {
        let text = text.into();
        if text != self.text {
            self.text = text;
            self.layout = None;
        }
    }

    /// Set the default text color for this layout.
    pub fn set_text_color(&mut self, color: impl Into<KeyOrValue<Color>>) {
        let color = color.into();
        if color != self.text_color {
            self.text_color = color;
            self.layout = None;
        }
    }

    /// Set the default font.
    ///
    /// The argument is a [`FontDescriptor`] or a [`Key<FontDescriptor>`] that
    /// can be resolved from the [`Env`].
    ///
    /// [`FontDescriptor`]: struct.FontDescriptor.html
    /// [`Env`]: struct.Env.html
    /// [`Key<FontDescriptor>`]: struct.Key.html
    pub fn set_font(&mut self, font: impl Into<KeyOrValue<FontDescriptor>>) {
        let font = font.into();
        if font != self.font {
            self.font = font;
            self.layout = None;
            self.text_size_override = None;
        }
    }

    /// Set the font size.
    ///
    /// This overrides the size in the [`FontDescriptor`] provided to [`set_font`].
    ///
    /// [`set_font`]: #method.set_font.html
    /// [`FontDescriptor`]: struct.FontDescriptor.html
    pub fn set_text_size(&mut self, size: impl Into<KeyOrValue<f64>>) {
        let size = size.into();
        if Some(&size) != self.text_size_override.as_ref() {
            self.text_size_override = Some(size);
            self.layout = None;
        }
    }

    /// Set the width at which to wrap words.
    ///
    /// You may pass `f64::INFINITY` to disable word wrapping
    /// (the default behaviour).
    pub fn set_wrap_width(&mut self, width: f64) {
        // 1e-4 is an arbitrary small-enough value that we don't care to rewrap
        if (width - self.wrap_width).abs() > 1e-4 {
            self.wrap_width = width;
            self.layout = None;
        }
    }

    /// Set the [`TextAlignment`] for this layout.
    ///
    /// [`TextAlignment`]: enum.TextAlignment.html
    pub fn set_text_alignment(&mut self, alignment: TextAlignment) {
        if self.alignment != alignment {
            self.alignment = alignment;
            self.layout = None;
        }
    }

    /// The size of the laid-out text.
    ///
    /// This is not meaningful until [`rebuild_if_needed`] has been called.
    ///
    /// [`rebuild_if_needed`]: #method.rebuild_if_needed
    pub fn size(&self) -> Size {
        self.layout
            .as_ref()
            .map(|layout| layout.size())
            .unwrap_or_default()
    }

    /// For a given `Point` (relative to this object's origin), returns index
    /// into the underlying text of the nearest grapheme boundary.
    pub fn text_position_for_point(&self, point: Point) -> usize {
        self.layout
            .as_ref()
            .map(|layout| layout.hit_test_point(point).idx)
            .unwrap_or_default()
    }

    /// Given the utf-8 position of a character boundary in the underlying text,
    /// return the `Point` (relative to this object's origin) representing the
    /// boundary of the containing grapheme.
    ///
    /// # Panics
    ///
    /// Panics if `text_pos` is not a character boundary.
    pub fn point_for_text_position(&self, text_pos: usize) -> Point {
        self.layout
            .as_ref()
            .map(|layout| layout.hit_test_text_position(text_pos).point)
            .unwrap_or_default()
    }

    /// Given a utf-8 range in the underlying text, return a `Vec` of `Rect`s
    /// representing the nominal bounding boxes of the text in that range.
    ///
    /// # Panics
    ///
    /// Panics if the range start or end is not a character boundary.
    pub fn rects_for_range(&self, range: Range<usize>) -> Vec<Rect> {
        self.layout
            .as_ref()
            .map(|layout| layout.rects_for_range(range))
            .unwrap_or_default()
    }

    /// Given the utf-8 position of a character boundary in the underlying text,
    /// return a `Line` suitable for drawing a vertical cursor at that boundary.
    pub fn cursor_line_for_text_position(&self, text_pos: usize) -> Line {
        self.layout
            .as_ref()
            .map(|layout| {
                let pos = layout.hit_test_text_position(text_pos);
                let line_metrics = layout.line_metric(pos.line).unwrap();
                let p1 = (pos.point.x, line_metrics.y_offset);
                let p2 = (pos.point.x, (line_metrics.y_offset + line_metrics.height));
                Line::new(p1, p2)
            })
            .unwrap_or_else(|| Line::new(Point::ZERO, Point::ZERO))
    }

    /// Called during the containing widgets `update` method; this text object
    /// will check to see if any used environment items have changed,
    /// and invalidate itself as needed.
    ///
    /// Returns `true` if the text item needs to be rebuilt.
    pub fn needs_rebuild_after_update(&mut self, ctx: &mut UpdateCtx) -> bool {
        if ctx.env_changed() && self.layout.is_some() {
            let rebuild = ctx.env_key_changed(&self.font)
                || ctx.env_key_changed(&self.text_color)
                || self
                    .text_size_override
                    .as_ref()
                    .map(|k| ctx.env_key_changed(k))
                    .unwrap_or(false);

            if rebuild {
                self.layout = None;
            }
        }
        self.layout.is_none()
    }

    /// Rebuild the inner layout as needed.
    ///
    /// This `TextLayout` object manages a lower-level layout object that may
    /// need to be rebuilt in response to changes to the text or attributes
    /// like the font.
    ///
    /// This method should be called whenever any of these things may have changed.
    /// A simple way to ensure this is correct is to always call this method
    /// as part of your widget's [`layout`] method.
    ///
    /// [`layout`]: trait.Widget.html#method.layout
    pub fn rebuild_if_needed(&mut self, factory: &mut PietText, env: &Env) {
        if self.layout.is_none() {
            let font = self.font.resolve(env);
            let color = self.text_color.resolve(env);
            let size_override = self.text_size_override.as_ref().map(|key| key.resolve(env));

            let descriptor = if let Some(size) = size_override {
                font.with_size(size)
            } else {
                font
            };

            self.layout = Some(
                factory
                    .new_text_layout(self.text.clone())
                    .max_width(self.wrap_width)
                    .alignment(self.alignment)
                    .font(descriptor.family.clone(), descriptor.size)
                    .default_attribute(descriptor.weight)
                    .default_attribute(descriptor.style)
                    .default_attribute(TextAttribute::TextColor(color))
                    .build()
                    .unwrap(),
            )
        }
    }

    ///  Draw the layout at the provided `Point`.
    ///
    ///  The origin of the layout is the top-left corner.
    ///
    ///  You must call [`rebuild_if_needed`] at some point before you first
    ///  call this method.
    ///
    ///  [`rebuild_if_needed`]: #method.rebuild_if_needed
    pub fn draw(&self, ctx: &mut PaintCtx, point: impl Into<Point>) {
        debug_assert!(
            self.layout.is_some(),
            "TextLayout::draw called without rebuilding layout object. Text was '{}'",
            &self.text
        );
        if let Some(layout) = self.layout.as_ref() {
            ctx.draw_text(layout, point);
        }
    }
}

impl std::fmt::Debug for TextLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("TextLayout")
            .field("font", &self.font)
            .field("text_size_override", &self.text_size_override)
            .field("text_color", &self.text_color)
            .field(
                "layout",
                if self.layout.is_some() {
                    &"Some"
                } else {
                    &"None"
                },
            )
            .finish()
    }
}
