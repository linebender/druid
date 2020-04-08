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

use crate::piet::{
    Color, FontBuilder, PietText, PietTextLayout, RenderContext, Text, TextLayout,
    TextLayoutBuilder, UnitPoint,
};
use crate::{
    theme, BoxConstraints, Data, Env, Event, EventCtx, KeyOrValue, LayoutCtx, LifeCycle,
    LifeCycleCtx, LocalizedString, PaintCtx, Point, Size, UpdateCtx, Widget,
};

// a fudgey way to get an approximate line height from a font size
const LINE_HEIGHT_FACTOR: f64 = 1.2;
// a fudgey way of figuring out where to put the baseline, relative to line height
const BASELINE_GUESS_FACTOR: f64 = 0.8;
// added padding between the edges of the widget and the text.
const LABEL_X_PADDING: f64 = 2.0;

/// The text for the label
pub enum LabelText<T> {
    /// Localized string that will be resolved through `Env`.
    Localized(LocalizedString<T>),
    /// Specific text
    Specific(String),
    /// The provided closure is called on update, and its return
    /// value is used as the text for the label.
    Dynamic(Dynamic<T>),
}

/// Text that is computed dynamically.
#[doc(hidden)]
pub struct Dynamic<T> {
    f: Box<dyn Fn(&T, &Env) -> String>,
    resolved: String,
}

/// A label that displays some text.
pub struct Label<T> {
    text: LabelText<T>,
    color: KeyOrValue<Color>,
    size: KeyOrValue<f64>,
}

impl<T: Data> Label<T> {
    /// Construct a new `Label` widget.
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
            color: theme::LABEL_COLOR.into(),
            size: theme::TEXT_SIZE_NORMAL.into(),
        }
    }

    /// Construct a new dynamic label.
    ///
    /// The contents of this label are generated from the data using a closure.
    ///
    /// This is provided as a convenience; a closure can also be passed to [`new`],
    /// but due to limitations of the implementation of that method, the types in
    /// the closure need to be annotated, which is not true for this method.
    ///
    /// # Examples
    ///
    /// The following are equivalent.
    ///
    /// ```
    /// use druid::Env;
    /// use druid::widget::Label;
    /// let button1: Label<u32> = Label::new(|data: &u32, _: &Env| format!("total is {}", data));
    /// let button2: Label<u32> = Label::dynamic(|data, _| format!("total is {}", data));
    /// ```
    ///
    /// [`new`]: #method.new
    pub fn dynamic(text: impl Fn(&T, &Env) -> String + 'static) -> Self {
        let text: LabelText<T> = text.into();
        Label::new(text)
    }

    /// Set text alignment.
    #[deprecated(since = "0.5.0", note = "Use an Align widget instead")]
    pub fn text_align(self, _align: UnitPoint) -> Self {
        self
    }

    /// Builder-style method for setting the text color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// [`Key<Color>`]: struct.Key.html
    pub fn with_text_color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        self.color = color.into();
        self
    }

    /// Builder-style method for setting the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// [`Key<f64>`]: struct.Key.html
    pub fn with_text_size(mut self, size: impl Into<KeyOrValue<f64>>) -> Self {
        self.size = size.into();
        self
    }

    /// Set a new text.
    ///
    /// Takes an already resolved string as input.
    ///
    /// If you're looking for full [`LabelText`] support,
    /// then you need to create a new [`Label`].
    ///
    /// [`Label`]: #method.new
    /// [`LabelText`]: enum.LabelText.html
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = LabelText::Specific(text.into());
    }

    /// Returns this label's current text.
    pub fn text(&self) -> &str {
        self.text.display_text()
    }

    /// Set the text color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// [`Key<Color>`]: struct.Key.html
    pub fn set_text_color(&mut self, color: impl Into<KeyOrValue<Color>>) {
        self.color = color.into();
    }

    /// Set the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// [`Key<f64>`]: struct.Key.html
    pub fn set_text_size(&mut self, size: impl Into<KeyOrValue<f64>>) {
        self.size = size.into();
    }

    fn get_layout(&mut self, t: &mut PietText, env: &Env) -> PietTextLayout {
        let font_name = env.get(theme::FONT_NAME);
        let font_size = self.size.resolve(env);

        // TODO: caching of both the format and the layout
        let font = t.new_font_by_name(font_name, font_size).build().unwrap();
        self.text
            .with_display_text(|text| t.new_text_layout(&font, &text).build().unwrap())
    }
}

impl<T> Dynamic<T> {
    fn resolve(&mut self, data: &T, env: &Env) -> bool {
        let new = (self.f)(data, env);
        let changed = new != self.resolved;
        self.resolved = new;
        changed
    }
}

impl<T: Data> LabelText<T> {
    /// Call callback with the text that should be displayed.
    pub fn with_display_text<V>(&self, mut cb: impl FnMut(&str) -> V) -> V {
        match self {
            LabelText::Specific(s) => cb(s.as_str()),
            LabelText::Localized(s) => cb(s.localized_str()),
            LabelText::Dynamic(s) => cb(s.resolved.as_str()),
        }
    }

    /// Return the current resolved text.
    pub fn display_text(&self) -> &str {
        match self {
            LabelText::Specific(s) => s.as_str(),
            LabelText::Localized(s) => s.localized_str(),
            LabelText::Dynamic(s) => s.resolved.as_str(),
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
            LabelText::Dynamic(s) => s.resolve(data, env),
        }
    }
}

impl<T: Data> Widget<T> for Label<T> {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.text.resolve(data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if !old_data.same(data) && self.text.resolve(data, env) {
            ctx.request_layout();
        }
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        env: &Env,
    ) -> Size {
        bc.debug_check("Label");

        let font_size = self.size.resolve(env);
        let text_layout = self.get_layout(layout_ctx.text(), env);
        bc.constrain(Size::new(
            text_layout.width() + 2. * LABEL_X_PADDING,
            font_size * LINE_HEIGHT_FACTOR,
        ))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        let font_size = self.size.resolve(env);
        let text_layout = self.get_layout(ctx.text(), env);
        let line_height = font_size * LINE_HEIGHT_FACTOR;

        // Find the origin for the text
        let origin = Point::new(LABEL_X_PADDING, line_height * BASELINE_GUESS_FACTOR);
        let color = self.color.resolve(env);

        ctx.draw_text(&text_layout, origin, &color);
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
        let f = Box::new(src);
        LabelText::Dynamic(Dynamic {
            f,
            resolved: String::default(),
        })
    }
}
