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

//! Common widgets.

mod align;
mod button;
mod checkbox;
mod container;
mod either;
mod env_scope;
mod flex;
mod identity_wrapper;
mod label;
mod list;
mod padding;
mod parse;
mod progress_bar;
mod radio;
mod scroll;
mod sized_box;
mod slider;
mod split;
mod stepper;
#[cfg(feature = "svg")]
#[cfg_attr(docsrs, doc(cfg(feature = "svg")))]
mod svg;
mod switch;
mod textbox;
mod widget_ext;

pub use align::Align;
pub use button::Button;
pub use checkbox::Checkbox;
pub use container::Container;
pub use either::Either;
pub use env_scope::EnvScope;
pub use flex::{Column, Flex, Row};
pub use identity_wrapper::IdentityWrapper;
pub use label::{Label, LabelText};
pub use list::{List, ListIter};
pub use padding::Padding;
pub use parse::Parse;
pub use progress_bar::ProgressBar;
pub use radio::{Radio, RadioGroup};
pub use scroll::Scroll;
pub use sized_box::SizedBox;
pub use slider::Slider;
pub use split::Split;
pub use stepper::Stepper;
#[cfg(feature = "svg")]
#[cfg_attr(docsrs, doc(cfg(feature = "svg")))]
pub use svg::{Svg, SvgData};
pub use switch::Switch;
pub use textbox::TextBox;
pub use widget_ext::WidgetExt;

use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};

use crate::kurbo::Size;
use crate::{BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx};

/// A unique identifier for a single widget.
// this is NonZeroU64 because we regularly store Option<WidgetId>
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct WidgetId(NonZeroU64);

/// The trait implemented by all widgets.
///
/// All appearance and behavior for a widget is encapsulated in an
/// object that implements this trait.
///
/// The trait is parametrized by a type (`T`) for associated data.
/// All trait methods are provided with access to this data, and
/// in the case of `event` the reference is mutable, so that events
/// can directly update the data.
///
/// Whenever the application data changes, the framework traverses
/// the widget hierarchy with an [`update`] method. The framework
/// needs to know whether the data has actually changed or not, which
/// is why `T` has a [`Data`] bound.
///
/// All the trait methods are provided with a corresponding context.
/// The widget can request things and cause actions by calling methods
/// on that context.
///
/// In addition, all trait methods are provided with an environment
/// ([`Env`](struct.Env.html)).
///
/// Container widgets will generally not call `Widget` methods directly
/// on their child widgets, but rather will own their widget wrapped in
/// a [`WidgetPod`], and call the corresponding method on that. The
/// `WidgetPod` contains state and logic for these traversals. On the
/// other hand, particularly light-weight containers might contain their
/// child `Widget` directly (when no layout or event flow logic is
/// needed), and in those cases will call these methods.
///
/// As a general pattern, container widgets will call the corresponding
/// `WidgetPod` method on all their children. The `WidgetPod` applies
/// logic to determine whether to recurse, as needed.
///
/// [`event`]: #tymethod.event
/// [`update`]: #tymethod.update
/// [`Data`]: trait.Data.html
/// [`WidgetPod`]: struct.WidgetPod.html
pub trait Widget<T> {
    /// Handle an event.
    ///
    /// A number of different events (in the [`Event`] enum) are handled in this
    /// method call. A widget can handle these events in a number of ways:
    /// requesting things from the [`EventCtx`], mutating the data, or submitting
    /// a [`Command`].
    ///
    /// [`Event`]: struct.Event.html
    /// [`EventCtx`]: struct.EventCtx.html
    /// [`Command`]: struct.Command.html
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env);

    /// Handle a change of data.
    ///
    /// This method is called whenever the data changes. When the appearance of
    /// the widget depends on data, call [`invalidate`] so that it's scheduled
    /// for repaint.
    ///
    /// The previous value of the data is provided in case the widget wants to
    /// compute a fine-grained delta. Before any paint operation, this method
    /// will be called with `None` for `old_data`. Thus, this method can also be
    /// used to build resources that will be retained for painting.
    ///
    /// [`invalidate`]: struct.UpdateCtx.html#method.invalidate

    // Consider a no-op default impl. One reason against is that containers might
    // inadvertently forget to propagate.
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env);

    /// Compute layout.
    ///
    /// A leaf widget should determine its size (subject to the provided
    /// constraints) and return it.
    ///
    /// A container widget will recursively call [`WidgetPod::layout`] on its
    /// child widgets, providing each of them an appropriate box constraint,
    /// compute layout, then call [`set_layout_rect`] on each of its children.
    /// Finally, it should return the size of the container. The container
    /// can recurse in any order, which can be helpful to, for example, compute
    /// the size of non-flex widgets first, to determine the amount of space
    /// available for the flex widgets.
    ///
    /// For efficiency, a container should only invoke layout of a child widget
    /// once, though there is nothing enforcing this.
    ///
    /// The layout strategy is strongly inspired by Flutter.
    ///
    /// [`WidgetPod::layout`]: struct.WidgetPod.html#method.layout
    /// [`set_layout_rect`]: struct.LayoutCtx.html#method.set_layout_rect
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size;

    /// Paint the widget appearance.
    ///
    /// The widget calls methods on the `render_ctx` field of the
    /// `paint_ctx` in order to paint its appearance. `paint_ctx` auto
    /// derefs to `render_ctx` for convenience.
    ///
    /// Container widgets can paint a background before recursing to their
    /// children, or annotations (for example, scrollbars) by painting
    /// afterwards. In addition, they can apply masks and transforms on
    /// the render context, which is especially useful for scrolling.
    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env);

    #[doc(hidden)]
    /// Get the identity of the widget; this is basically only implemented by
    /// `IdentityWrapper`. Widgets should not implement this on their own.
    fn id(&self) -> Option<WidgetId> {
        None
    }
}

impl WidgetId {
    /// Allocate a new, unique widget id.
    ///
    /// Do note that if we create 4 billion widgets there may be a collision.
    pub(crate) fn next() -> WidgetId {
        use crate::shell::Counter;
        static WIDGET_ID_COUNTER: Counter = Counter::new();
        WidgetId(WIDGET_ID_COUNTER.next_nonzero())
    }
}

// TODO: explore getting rid of this (ie be consistent about using
// `dyn Widget` only).
impl<T> Widget<T> for Box<dyn Widget<T>> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.deref_mut().event(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        self.deref_mut().update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.deref_mut().layout(ctx, bc, data, env)
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.deref_mut().paint(paint_ctx, data, env);
    }

    fn id(&self) -> Option<WidgetId> {
        self.deref().id()
    }
}
