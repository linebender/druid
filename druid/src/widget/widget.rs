// Copyright 2018 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};

use super::prelude::*;
use crate::debug_state::DebugState;
use crate::widget::Axis;

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
/// If you set a `WidgetId` directly, you are responsible for ensuring that it
/// is unique in time. That is: only one widget can exist with a given id at a
/// given time.
///
/// [`Target`]: crate::Target
/// [`WidgetPod`]: crate::WidgetPod
/// [`WidgetExt::with_id`]: super::WidgetExt::with_id
/// [`IdentityWrapper`]: super::IdentityWrapper
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
/// [`event`]: Widget::event
/// [`update`]: Widget::update
/// [`WidgetPod`]: crate::WidgetPod
pub trait Widget<T> {
    /// Handle an event.
    ///
    /// A number of different events (in the [`Event`] enum) are handled in this
    /// method call. A widget can handle these events in a number of ways:
    /// requesting things from the [`EventCtx`], mutating the data, or submitting
    /// a [`Command`].
    ///
    /// [`Command`]: crate::Command
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
    /// [`Command`]: crate::Command
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
    /// [`env_changed`]: UpdateCtx::env_changed
    /// [`env_key_changed`]: UpdateCtx::env_key_changed
    /// [`request_paint`]: UpdateCtx::request_paint
    /// [`request_layout`]: UpdateCtx::request_layout
    /// [`layout`]: Widget::layout
    /// [`paint`]: Widget::paint
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env);

    /// Compute layout.
    ///
    /// A leaf widget should determine its size (subject to the provided
    /// constraints) and return it.
    ///
    /// A container widget will recursively call [`WidgetPod::layout`] on its
    /// child widgets, providing each of them an appropriate box constraint,
    /// compute layout, then call [`set_origin`] on each of its children.
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
    /// [`WidgetPod::layout`]: crate::WidgetPod::layout
    /// [`set_origin`]: crate::WidgetPod::set_origin
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

    #[doc(hidden)]
    /// Get the (abridged) type name of the widget for debugging purposes.
    /// You should not override this method.
    fn short_type_name(&self) -> &'static str {
        let name = self.type_name();
        name.split('<')
            .next()
            .unwrap_or(name)
            .split("::")
            .last()
            .unwrap_or(name)
    }

    #[doc(hidden)]
    /// From the current data, get a best-effort description of the state of
    /// this widget and its children for debugging purposes.
    fn debug_state(&self, data: &T) -> DebugState {
        #![allow(unused_variables)]
        DebugState {
            display_name: self.short_type_name().to_string(),
            ..Default::default()
        }
    }

    /// Computes max intrinsic/preferred dimension of a widget on the provided axis.
    ///
    /// Max intrinsic/preferred dimension is the dimension the widget could take, provided infinite
    /// constraint on that axis.
    ///
    /// If axis == Axis::Horizontal, widget is being asked to calculate max intrinsic width.
    /// If axis == Axis::Vertical, widget is being asked to calculate max intrinsic height.
    ///
    /// Box constraints must be honored in intrinsics computation.
    ///
    /// AspectRatioBox is an example where constraints are honored. If height is finite, max intrinsic
    /// width is *height * ratio*.
    /// Only when height is infinite, child's max intrinsic width is calculated.
    ///
    /// Intrinsic is a *could-be* value. It's the value a widget *could* have given infinite constraints.
    /// This does not mean the value returned by layout() would be the same.
    ///
    /// This method **must** return a finite value.
    fn compute_max_intrinsic(
        &mut self,
        axis: Axis,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> f64 {
        match axis {
            Axis::Horizontal => self.layout(ctx, bc, data, env).width,
            Axis::Vertical => self.layout(ctx, bc, data, env).height,
        }
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
        let id = u64::MAX - raw as u64;
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

    fn debug_state(&self, data: &T) -> DebugState {
        self.deref().debug_state(data)
    }

    fn compute_max_intrinsic(
        &mut self,
        axis: Axis,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> f64 {
        self.deref_mut()
            .compute_max_intrinsic(axis, ctx, bc, data, env)
    }
}
