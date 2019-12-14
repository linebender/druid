// Copyright 2019 The xi-editor Authors.
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

//! A label widget.

use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Size, UpdateCtx,
    Widget,
};

use crate::piet::{
    FontBuilder, PietFont, PietText, Text, TextLayout, TextLayoutBuilder, UnitPoint,
};

use crate::localization::LocalizedString;
use crate::theme;
use crate::{Point, RenderContext};

/// The text for the label
pub enum LabelText<T> {
    /// Localized string that will be resolved through `Env`.
    Localized(LocalizedString<T>),
    /// Specific text
    Specific(String),
    /// The provided closure is called on update, and its return
    /// value is used as the text for the label.
    Dynamic(Box<dyn Fn(&T, &Env) -> String>),
}

/// WordBreak sets whether line breaks if text doesn't fit in a single line.
/// Values correspond to a [word-break](https://developer.mozilla.org/en-US/docs/Web/CSS/word-break)
/// CSS property.
#[derive(PartialEq)]
enum WordBreak {
    Normal,
    KeepAll,
}

/// A label that displays some text.
pub struct Label<T> {
    text: LabelText<T>,
    align: UnitPoint,
    word_break: WordBreak,
}

impl<T: Data> Label<T> {
    /// Construct a new Label widget.
    ///
    /// ```
    /// use druid::LocalizedString;
    /// use druid::widget::Label;
    ///
    /// // Construct a new Label using static string.
    /// let _: Label<u32> = Label::new("Hello world");
    ///
    /// // Construct a new Label using localized string.
    /// let text = LocalizedString::new("hello-counter").with_arg("count", |data: &u32, _env| (*data).into());
    /// let _: Label<u32> = Label::new(text);
    ///
    /// // Construct a new dynamic Label. Text will be updated when data changes.
    /// let _: Label<u32> = Label::new(|data: &u32, _env: &_| format!("Hello world: {}", data));
    /// ```
    pub fn new(text: impl Into<LabelText<T>>) -> Self {
        let text = text.into();
        Self {
            text,
            align: UnitPoint::LEFT,
            word_break: WordBreak::KeepAll,
        }
    }

    /// Set text alignment.
    pub fn align(mut self, align: UnitPoint) -> Self {
        self.align = align;
        self
    }

    /// Break words into multiple lines if the text doesn't fit in one line.
    pub fn break_words(mut self) -> Self {
        self.word_break = WordBreak::Normal;
        self
    }

    fn get_font(&mut self, t: &mut PietText, env: &Env) -> (PietFont, f64) {
        let font_name = env.get(theme::FONT_NAME);
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);

        (
            t.new_font_by_name(font_name, font_size).build().unwrap(),
            font_size,
        )
    }

    fn collect_lines<'a>(
        &self,
        text: &'a str,
        max_width: f64,
        mut count: impl FnMut(&str) -> f64,
    ) -> Vec<&'a str> {
        if self.word_break == WordBreak::KeepAll {
            return vec![text];
        }

        let mut lines = vec![];

        let mut line = (0, 0);
        let mut word = (0, 0);
        let mut line_width = 0.0;
        let text_len = text.chars().count();

        for (i, c) in text.chars().enumerate() {
            word.1 += 1;

            let last_char = i + 1 == text_len;
            if c == ' ' || c == '-' || c == '\t' || last_char {
                // Word has ended
                let word_width = count(&text[word.0..word.1]);

                let line_non_empty = (line.1 - line.0) > 0;
                if ((line_width + word_width) > max_width) && line_non_empty {
                    // Word was wrapped onto the next line
                    lines.push(&text[line.0..line.1]);

                    line_width = word_width;
                    line = word;
                } else {
                    // Word fits onto the same line
                    line_width += word_width;
                    line.1 = word.1;
                }

                word.0 = word.1;
            }
        }

        if line.1 - line.0 > 0 {
            // Include last line
            lines.push(&text[line.0..line.1]);
        }

        lines
    }
}

impl<T: Data> LabelText<T> {
    /// Call callback with the text that should be displayed.
    pub fn with_display_text<V>(&self, data: &T, env: &Env, mut cb: impl FnMut(&str) -> V) -> V {
        match self {
            LabelText::Specific(s) => cb(s.as_str()),
            LabelText::Localized(s) => cb(s.localized_str()),
            LabelText::Dynamic(f) => cb((f)(data, env).as_str()),
        }
    }

    /// Update the localization, if necessary.
    /// This ensures that localized strings are up to date.
    ///
    /// Returns `true` if the string has changed.
    pub fn resolve(&mut self, data: &T, env: &Env) -> bool {
        match self {
            LabelText::Specific(_) => false,
            LabelText::Localized(s) => s.resolve(data, env),
            LabelText::Dynamic(_s) => false,
        }
    }
}

impl<T: Data> Widget<T> for Label<T> {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&T>, data: &T, env: &Env) {
        if self.text.resolve(data, env) {
            ctx.invalidate();
        }
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        bc.debug_check("Label");

        let (font, font_size) = self.get_font(layout_ctx.text(), env);

        self.text.with_display_text(data, env, |text| {
            // Calculate the amount of lines needed for the text
            let lines = self.collect_lines(text, bc.max.width, |word| {
                layout_ctx
                    .text()
                    .new_text_layout(&font, word)
                    .build()
                    .unwrap()
                    .width()
            });

            let width = if lines.len() > 1 {
                bc.max.width
            } else {
                layout_ctx
                    .text()
                    .new_text_layout(&font, text)
                    .build()
                    .unwrap()
                    .width()
            };

            // This magical 1.2 constant helps center the text vertically in the rect it's given
            let height = (lines.len() as f64) * font_size * 1.2;
            bc.constrain(Size::new(width, height))
        })
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        // TODO: shall align happen outside of Label widget?
        // Find the origin for the text
        // let mut origin = self.align.resolve(Rect::from_origin_size(Point::ORIGIN, base_state.size()));
        let mut origin = Point::ORIGIN;

        let (font, font_size) = self.get_font(paint_ctx.text(), env);

        self.text.with_display_text(data, env, |text| {
            let lines = self.collect_lines(text, base_state.size().width, |word| {
                paint_ctx
                    .text()
                    .new_text_layout(&font, word)
                    .build()
                    .unwrap()
                    .width()
            });

            origin.y -= font_size / 4.0;

            for line in lines {
                origin.y += 1.2 * font_size;

                let line_layout = paint_ctx
                    .text()
                    .new_text_layout(&font, &line)
                    .build()
                    .unwrap();
                paint_ctx.draw_text(&line_layout, origin, &env.get(theme::LABEL_COLOR));
            }
        });
    }
}

impl<T> From<String> for LabelText<T> {
    fn from(src: String) -> LabelText<T> {
        LabelText::Specific(src)
    }
}

impl<T> From<&str> for LabelText<T> {
    fn from(src: &str) -> LabelText<T> {
        LabelText::Specific(src.to_string())
    }
}

impl<T> From<LocalizedString<T>> for LabelText<T> {
    fn from(src: LocalizedString<T>) -> LabelText<T> {
        LabelText::Localized(src)
    }
}

impl<T, F: Fn(&T, &Env) -> String + 'static> From<F> for LabelText<T> {
    fn from(src: F) -> LabelText<T> {
        LabelText::Dynamic(Box::new(src))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_lines_no_wrapping() {
        let label: Label<String> = Label::new("");
        let lines = label.collect_lines("hello world", 1.0, |word| word.len() as f64);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn test_collect_lines_word_wrapping() {
        let label: Label<String> = Label::new("").break_words();
        let lines = label.collect_lines("hello my world again 2", 10.0, |word| word.len() as f64);
        assert_eq!(lines, vec!["hello my ", "world ", "again 2"]);

        let lines = label.collect_lines("hello", 1.0, |word| word.len() as f64);
        assert_eq!(lines, vec!["hello"]);
    }
}
