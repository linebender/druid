// Copyright 2019 The Druid Authors.
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

use crate::piet::{Color, PietText};
use crate::widget::prelude::*;
use crate::{
    BoxConstraints, Data, FontDescriptor, KeyOrValue, LocalizedString, Point, Size, TextAlignment,
    TextLayout,
};

// added padding between the edges of the widget and the text.
const LABEL_X_PADDING: f64 = 2.0;

/// The text for the label.
///
/// This can be one of three things; either a `String`, a [`LocalizedString`],
/// or a closure with the signature, `Fn(&T, &Env) -> String`, where `T` is
/// the `Data` at this point in the tree.
///
/// [`LocalizedString`]: ../struct.LocalizedString.html
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
    layout: TextLayout,
    line_break_mode: LineBreaking,
    // if our text is manually changed we need to rebuild the layout
    // before using it again.
    needs_rebuild: bool,
}

/// Options for handling lines that are too wide for the label.
#[derive(Debug, Clone, Copy, PartialEq, Data)]
pub enum LineBreaking {
    /// Lines are broken at word boundaries.
    WordWrap,
    /// Lines are truncated to the width of the label.
    Clip,
    /// Lines overflow the label.
    Overflow,
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
        let layout = TextLayout::new(text.display_text());
        Self {
            text,
            layout,
            line_break_mode: LineBreaking::Clip,
            needs_rebuild: true,
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
    /// let label1: Label<u32> = Label::new(|data: &u32, _: &Env| format!("total is {}", data));
    /// let label2: Label<u32> = Label::dynamic(|data, _| format!("total is {}", data));
    /// ```
    ///
    /// [`new`]: #method.new
    pub fn dynamic(text: impl Fn(&T, &Env) -> String + 'static) -> Self {
        let text: LabelText<T> = text.into();
        Label::new(text)
    }

    /// Builder-style method for setting the text color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// [`Key<Color>`]: ../struct.Key.html
    pub fn with_text_color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        self.set_text_color(color);
        self
    }

    /// Builder-style method for setting the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// [`Key<f64>`]: ../struct.Key.html
    pub fn with_text_size(mut self, size: impl Into<KeyOrValue<f64>>) -> Self {
        self.set_text_size(size);
        self
    }

    /// Builder-style method for setting the font.
    ///
    /// The argument can be a [`FontDescriptor`] or a [`Key<FontDescriptor>`]
    /// that refers to a font defined in the [`Env`].
    ///
    /// [`Env`]: ../struct.Env.html
    /// [`FontDescriptor`]: ../struct.FontDescriptor.html
    /// [`Key<FontDescriptor>`]: ../struct.Key.html
    pub fn with_font(mut self, font: impl Into<KeyOrValue<FontDescriptor>>) -> Self {
        self.set_font(font);
        self
    }

    /// Builder-style method to set the [`LineBreaking`] behaviour.
    ///
    /// [`LineBreaking`]: enum.LineBreaking.html
    pub fn with_line_break_mode(mut self, mode: LineBreaking) -> Self {
        self.set_line_break_mode(mode);
        self
    }

    /// Builder-style method to set the [`TextAlignment`].
    ///
    /// [`TextAlignment`]: enum.TextAlignment.html
    pub fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        self.set_text_alignment(alignment);
        self
    }

    /// Set the label's text.
    ///
    /// If you change this property, you are responsible for calling
    /// [`request_layout`] to ensure the label is updated.
    ///
    /// [`request_layout`]: ../struct.EventCtx.html#method.request_layout
    pub fn set_text(&mut self, text: impl Into<LabelText<T>>) {
        self.text = text.into();
        self.needs_rebuild = true;
    }

    /// Returns this label's current text.
    pub fn text(&self) -> &str {
        self.text.display_text()
    }

    /// Set the text color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// If you change this property, you are responsible for calling
    /// [`request_layout`] to ensure the label is updated.
    ///
    /// [`request_layout`]: ../struct.EventCtx.html#method.request_layout
    /// [`Key<Color>`]: ../struct.Key.html
    pub fn set_text_color(&mut self, color: impl Into<KeyOrValue<Color>>) {
        self.layout.set_text_color(color);
        self.needs_rebuild = true;
    }

    /// Set the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// If you change this property, you are responsible for calling
    /// [`request_layout`] to ensure the label is updated.
    ///
    /// [`request_layout`]: ../struct.EventCtx.html#method.request_layout
    /// [`Key<f64>`]: ../struct.Key.html
    pub fn set_text_size(&mut self, size: impl Into<KeyOrValue<f64>>) {
        self.layout.set_text_size(size);
        self.needs_rebuild = true;
    }

    /// Set the font.
    ///
    /// The argument can be a [`FontDescriptor`] or a [`Key<FontDescriptor>`]
    /// that refers to a font defined in the [`Env`].
    ///
    /// If you change this property, you are responsible for calling
    /// [`request_layout`] to ensure the label is updated.
    ///
    /// [`request_layout`]: ../struct.EventCtx.html#method.request_layout
    /// [`Env`]: ../struct.Env.html
    /// [`FontDescriptor`]: ../struct.FontDescriptor.html
    /// [`Key<FontDescriptor>`]: ../struct.Key.html
    pub fn set_font(&mut self, font: impl Into<KeyOrValue<FontDescriptor>>) {
        self.layout.set_font(font);
        self.needs_rebuild = true;
    }

    /// Set the [`LineBreaking`] behaviour.
    ///
    /// If you change this property, you are responsible for calling
    /// [`request_layout`] to ensure the label is updated.
    ///
    /// [`request_layout`]: ../struct.EventCtx.html#method.request_layout
    /// [`LineBreaking`]: enum.LineBreaking.html
    pub fn set_line_break_mode(&mut self, mode: LineBreaking) {
        self.line_break_mode = mode;
    }

    /// Set the [`TextAlignment`] for this layout.
    ///
    /// [`TextAlignment`]: enum.TextAlignment.html
    pub fn set_text_alignment(&mut self, alignment: TextAlignment) {
        self.layout.set_text_alignment(alignment);
        self.needs_rebuild = true;
    }

    fn rebuild_if_needed(&mut self, factory: &mut PietText, data: &T, env: &Env) {
        if self.needs_rebuild {
            self.text.resolve(data, env);
            self.layout.set_text(self.text.display_text());
            self.layout.rebuild_if_needed(factory, env);
            self.needs_rebuild = false;
        }
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

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &T, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, _env: &Env) {
        //FIXME: this should also be checking if anything in the env has changed
        if !old_data.same(data) {
            self.needs_rebuild = true;
            ctx.request_layout();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Label");

        let width = match self.line_break_mode {
            LineBreaking::WordWrap => bc.max().width - LABEL_X_PADDING * 2.0,
            _ => f64::INFINITY,
        };

        self.rebuild_if_needed(&mut ctx.text(), data, env);
        self.layout.set_wrap_width(width);

        let mut text_size = self.layout.size();
        text_size.width += 2. * LABEL_X_PADDING;
        bc.constrain(text_size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, _env: &Env) {
        let origin = Point::new(LABEL_X_PADDING, 0.0);
        let label_size = ctx.size();

        if self.line_break_mode == LineBreaking::Clip {
            ctx.clip(label_size.to_rect());
        }
        self.layout.draw(ctx, origin)
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
