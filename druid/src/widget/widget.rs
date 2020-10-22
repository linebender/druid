// Copyright 2018 The Druid Authors.
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

use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};

use super::prelude::*;

/// A unique identifier for a single [`Widget`].
///
/// `WidgetId`s are generated automatically for all widgets that participate
/// in layout. More specifically, each [`WidgetPod`] has a unique `WidgetId`.
///
/// These ids are used internally to route events, and can be used to communicate
/// between widgets, by submitting a command (as with [`EventCtx::submit_command`])
/// and passing a `WidgetId` as the [`Target`].
///
/// A widget can retrieve its id via methods on the various contexts, such as
/// [`LifeCycleCtx::widget_id`].
///
/// ## Explicit `WidgetId`s.
///
/// Sometimes, you may want to know a widget's id when constructing the widget.
/// You can give a widget an _explicit_ id by wrapping it in an [`IdentityWrapper`]
/// widget, or by using the [`WidgetExt::with_id`] convenience method.
///
/// If you set a `WidgetId` directly, you are resposible for ensuring that it
/// is unique in time. That is: only one widget can exist with a given id at a
/// given time.
///
/// [`Widget`]: trait.Widget.html
/// [`EventCtx::submit_command`]: struct.EventCtx.html#method.submit_command
/// [`Target`]: enum.Target.html
/// [`WidgetPod`]: struct.WidgetPod.html
/// [`LifeCycleCtx::widget_id`]: struct.LifeCycleCtx.html#method.widget_id
/// [`WidgetExt::with_id`]: trait.WidgetExt.html#method.with_id
/// [`IdentityWrapper`]: widget/struct.IdentityWrapper.html
// this is NonZeroU64 because we regularly store Option<WidgetId>
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct WidgetId(NonZeroU64);

/// The trait implemented by all widgets.
///
/// All appearance and behavior for a widget is encapsulated in an
/// object that implements this trait.
///
/// The trait is parametrized by a type (`T`) for associated data.
/// All trait methods are provided with access to this data, and
/// in the case of [`event`] the reference is mutable, so that events
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
/// ([`Env`]).
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
/// [`Env`]: struct.Env.html
/// [`WidgetPod`]: struct.WidgetPod.html
pub trait Widget<T> {
    /// Handle an event.
    ///
    /// A number of different events (in the [`Event`] enum) are handled in this
    /// method call. A widget can handle these events in a number of ways:
    /// requesting things from the [`EventCtx`], mutating the data, or submitting
    /// a [`Command`].
    ///
    /// [`Event`]: enum.Event.html
    /// [`EventCtx`]: struct.EventCtx.html
    /// [`Command`]: struct.Command.html
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env);

    /// Handle a life cycle notification.
    ///
    /// This method is called to notify your widget of certain special events,
    /// (available in the [`LifeCycle`] enum) that are generally related to
    /// changes in the widget graph or in the state of your specific widget.
    ///
    /// A widget is not expected to mutate the application state in response
    /// to these events, but only to update its own internal state as required;
    /// if a widget needs to mutate data, it can submit a [`Command`] that will
    /// be executed at the next opportunity.
    ///
    /// [`LifeCycle`]: enum.LifeCycle.html
    /// [`LifeCycleCtx`]: struct.LifeCycleCtx.html
    /// [`Command`]: struct.Command.html
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env);

    /// Update the widget's appearance in response to a change in the app's
    /// [`Data`] or [`Env`].
    ///
    /// This method is called whenever the data or environment changes.
    /// When the appearance of the widget needs to be updated in response to
    /// these changes, you can call [`request_paint`] or [`request_layout`] on
    /// the provided [`UpdateCtx`] to schedule calls to [`paint`] and [`layout`]
    /// as required.
    ///
    /// The previous value of the data is provided in case the widget wants to
    /// compute a fine-grained delta; you should try to only request a new
    /// layout or paint pass if it is actually required.
    ///
    /// To determine if the [`Env`] has changed, you can call [`env_changed`]
    /// on the provided [`UpdateCtx`]; you can then call [`env_key_changed`]
    /// with any keys that are used in your widget, to see if they have changed;
    /// you can then request layout or paint as needed.
    ///
    /// [`Data`]: trait.Data.html
    /// [`Env`]: struct.Env.html
    /// [`UpdateCtx`]: struct.UpdateCtx.html
    /// [`env_changed`]: struct.UpdateCtx.html#method.env_changed
    /// [`env_key_changed`]: struct.UpdateCtx.html#method.env_changed
    /// [`request_paint`]: struct.UpdateCtx.html#method.request_paint
    /// [`request_layout`]: struct.UpdateCtx.html#method.request_layout
    /// [`layout`]: #tymethod.layout
    /// [`paint`]: #tymethod.paint
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env);

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
    /// [`set_layout_rect`]: struct.WidgetPod.html#method.set_layout_rect
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size;

    /// Paint the widget appearance.
    ///
    /// The [`PaintCtx`] derefs to something that implements the [`RenderContext`]
    /// trait, which exposes various methods that the widget can use to paint
    /// its appearance.
    ///
    /// Container widgets can paint a background before recursing to their
    /// children, or annotations (for example, scrollbars) by painting
    /// afterwards. In addition, they can apply masks and transforms on
    /// the render context, which is especially useful for scrolling.
    ///
    /// [`PaintCtx`]: struct.PaintCtx.html
    /// [`RenderContext`]: trait.RenderContext.html
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env);

    #[doc(hidden)]
    /// Get the identity of the widget; this is basically only implemented by
    /// `IdentityWrapper`. Widgets should not implement this on their own.
    fn id(&self) -> Option<WidgetId> {
        None
    }

    #[doc(hidden)]
    /// Get the (verbose) type name of the widget for debugging purposes.
    /// You should not override this method.
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl WidgetId {
    /// Allocate a new, unique `WidgetId`.
    ///
    /// All widgets are assigned ids automatically; you should only create
    /// an explicit id if you need to know it ahead of time, for instance
    /// if you want two sibling widgets to know each others' ids.
    ///
    /// You must ensure that a given `WidgetId` is only ever used for one
    /// widget at a time.
    pub fn next() -> WidgetId {
        use crate::shell::Counter;
        static WIDGET_ID_COUNTER: Counter = Counter::new();
        WidgetId(WIDGET_ID_COUNTER.next_nonzero())
    }

    /// Create a reserved `WidgetId`, suitable for reuse.
    ///
    /// The caller is responsible for ensuring that this ID is in fact assigned
    /// to a single widget at any time, or your code may become haunted.
    ///
    /// The actual inner representation of the returned `WidgetId` will not
    /// be the same as the raw value that is passed in; it will be
    /// `u64::max_value() - raw`.
    #[allow(unsafe_code)]
    pub const fn reserved(raw: u16) -> WidgetId {
        let id = u64::max_value() - raw as u64;
        // safety: by construction this can never be zero.
        WidgetId(unsafe { std::num::NonZeroU64::new_unchecked(id) })
    }

    pub(crate) fn to_raw(self) -> u64 {
        self.0.into()
    }
}

impl<T> Widget<T> for Box<dyn Widget<T>> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.deref_mut().event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.deref_mut().lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.deref_mut().update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.deref_mut().layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.deref_mut().paint(ctx, data, env);
    }

    fn id(&self) -> Option<WidgetId> {
        self.deref().id()
    }

    fn type_name(&self) -> &'static str {
        self.deref().type_name()
    }
}
