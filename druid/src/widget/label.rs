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
    ArcStr, BoxConstraints, Data, FontDescriptor, KeyOrValue, LocalizedString, Point, Size,
    TextAlignment, TextLayout,
};

// added padding between the edges of the widget and the text.
const LABEL_X_PADDING: f64 = 2.0;

/// The text for the label.
///
/// This can be one of three things; either a [`ArcStr`], a [`LocalizedString`],
/// or a closure with the signature, `Fn(&T, &Env) -> String`, where `T` is
/// the [`Data`] at this point in the tree.
///
/// [`ArcStr`]: ../type.ArcStr.html
/// [`LocalizedString`]: ../struct.LocalizedString.html
/// [`Data`]: ../trait.Data.html
pub enum LabelText<T> {
    /// Localized string that will be resolved through `Env`.
    Localized(LocalizedString<T>),
    /// Static text.
    Static(Static),
    /// The provided closure is called on update, and its return
    /// value is used as the text for the label.
    Dynamic(Dynamic<T>),
}

/// Text that is computed dynamically.
pub struct Dynamic<T> {
    f: Box<dyn Fn(&T, &Env) -> String>,
    resolved: ArcStr,
}

/// Static text.
pub struct Static {
    /// The text.
    string: ArcStr,
    /// Whether or not the `resolved` method has been called yet.
    ///
    /// We want to return `true` from that method when it is first called,
    /// so that callers will know to retrieve the text. This matches
    /// the behaviour of the other variants.
    resolved: bool,
}

/// A label that draws some text.
///
/// A label is the easiest way to display text in Druid. A label is instantiated
/// with some [`LabelText`] type, such as an [`ArcStr`] or a [`LocalizedString`],
/// and also has methods for setting the default font, font-size, text color,
/// and other attributes.
///
/// In addition to being a [`Widget`], `Label` is also regularly used as a
/// component in other widgets that wish to display text; to facilitate this
/// it has a [`draw_at`] method that allows the caller to easily draw the label's
/// text at the desired position on screen.
///
/// # Examples
///
/// Make a label to say something **very** important:
///
/// ```
/// # use druid::widget::{Label, SizedBox};
/// # use druid::*;
///
/// let font = FontDescriptor::new(FontFamily::SYSTEM_UI)
///     .with_weight(FontWeight::BOLD)
///     .with_size(48.0);
///
/// let important_label = Label::new("WATCH OUT!")
///     .with_font(font)
///     .with_text_color(Color::rgb(1.0, 0.2, 0.2));
/// # // our data type T isn't known; this is just a trick for the compiler
/// # // to keep our example clean
/// # let _ = SizedBox::<()>::new(important_label);
/// ```
///
/// [`ArcStr`]: ../type.ArcStr.html
/// [`LabelText`]: struct.LabelText.html
/// [`LocalizedString`]: ../struct.LocalizedString.html
/// [`draw_at`]: #method.draw_at
/// [`Widget`]: ../trait.Widget.html
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
            line_break_mode: LineBreaking::Overflow,
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
        self.needs_rebuild = true;
    }

    /// Set the [`TextAlignment`] for this layout.
    ///
    /// [`TextAlignment`]: enum.TextAlignment.html
    pub fn set_text_alignment(&mut self, alignment: TextAlignment) {
        self.layout.set_text_alignment(alignment);
        self.needs_rebuild = true;
    }

    /// Draw this label's text at the provided `Point`, without internal padding.
    ///
    /// This is a convenience for widgets that want to use Label as a way
    /// of managing a dynamic or localized string, but want finer control
    /// over where the text is drawn.
    pub fn draw_at(&self, ctx: &mut PaintCtx, origin: impl Into<Point>) {
        self.layout.draw(ctx, origin)
    }

    fn rebuild_if_needed(&mut self, factory: &mut PietText, data: &T, env: &Env) {
        if self.needs_rebuild || self.layout.needs_rebuild() {
            self.text.resolve(data, env);
            self.layout.set_text(self.text.display_text());
            self.layout.rebuild_if_needed(factory, env);
            self.needs_rebuild = false;
        }
    }
}

impl Static {
    fn new(s: ArcStr) -> Self {
        Static {
            string: s,
            resolved: false,
        }
    }

    fn resolve(&mut self) -> bool {
        let is_first_call = self.resolved;
        self.resolved = true;
        is_first_call
    }
}

impl<T> Dynamic<T> {
    fn resolve(&mut self, data: &T, env: &Env) -> bool {
        let new = (self.f)(data, env);
        let changed = new.as_str() != self.resolved.as_ref();
        self.resolved = new.into();
        changed
    }
}

impl<T: Data> LabelText<T> {
    /// Call callback with the text that should be displayed.
    pub fn with_display_text<V>(&self, mut cb: impl FnMut(&str) -> V) -> V {
        match self {
            LabelText::Static(s) => cb(&s.string),
            LabelText::Localized(s) => cb(&s.localized_str()),
            LabelText::Dynamic(s) => cb(&s.resolved),
        }
    }

    /// Return the current resolved text.
    pub fn display_text(&self) -> ArcStr {
        match self {
            LabelText::Static(s) => s.string.clone(),
            LabelText::Localized(s) => s.localized_str(),
            LabelText::Dynamic(s) => s.resolved.clone(),
        }
    }

    /// Update the localization, if necessary.
    /// This ensures that localized strings are up to date.
    ///
    /// Returns `true` if the string has changed.
    pub fn resolve(&mut self, data: &T, env: &Env) -> bool {
        match self {
            LabelText::Static(s) => s.resolve(),
            LabelText::Localized(s) => s.resolve(data, env),
            LabelText::Dynamic(s) => s.resolve(data, env),
        }
    }
}

impl<T: Data> Widget<T> for Label<T> {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &T, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, _env: &Env) {
        if !old_data.same(data) | self.layout.needs_rebuild_after_update(ctx) {
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

        self.layout.set_wrap_width(width);
        self.rebuild_if_needed(&mut ctx.text(), data, env);

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
        self.draw_at(ctx, origin)
    }
}

impl<T> From<String> for LabelText<T> {
    fn from(src: String) -> LabelText<T> {
        LabelText::Static(Static::new(src.into()))
    }
}

impl<T> From<&str> for LabelText<T> {
    fn from(src: &str) -> LabelText<T> {
        LabelText::Static(Static::new(src.into()))
    }
}

impl<T> From<ArcStr> for LabelText<T> {
    fn from(string: ArcStr) -> LabelText<T> {
        LabelText::Static(Static::new(string))
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
            resolved: ArcStr::from(""),
        })
    }
}
