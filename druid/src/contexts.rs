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

//! The context types that are passed into various widget methods.

use std::ops::{Deref, DerefMut};
use std::time::Instant;

use log;

use crate::core::{BaseState, CommandQueue, FocusChange};
use crate::piet::Piet;
use crate::piet::RenderContext;
use crate::{
    Affine, Command, Cursor, Rect, Size, Target, Text, TimerToken, WidgetId, WinCtx, WindowHandle,
    WindowId,
};

/// A mutable context provided to event handling methods of widgets.
///
/// Widgets should call [`invalidate`] whenever an event causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`invalidate`]: #method.invalidate
pub struct EventCtx<'a, 'b> {
    // Note: there's a bunch of state that's just passed down, might
    // want to group that into a single struct.
    pub(crate) win_ctx: &'a mut dyn WinCtx<'b>,
    pub(crate) cursor: &'a mut Option<Cursor>,
    /// Commands submitted to be run after this event.
    pub(crate) command_queue: &'a mut CommandQueue,
    pub(crate) window_id: WindowId,
    // TODO: migrate most usage of `WindowHandle` to `WinCtx` instead.
    pub(crate) window: &'a WindowHandle,
    pub(crate) base_state: &'a mut BaseState,
    pub(crate) focus_widget: Option<WidgetId>,
    pub(crate) had_active: bool,
    pub(crate) is_handled: bool,
    pub(crate) is_root: bool,
}

/// A mutable context provided to the [`lifecycle`] method on widgets.
///
/// Certain methods on this context are only meaningful during the handling of
/// specific lifecycle events; for instance [`register_child`]
/// should only be called while handling [`LifeCycle::Register`].
///
/// [`lifecycle`]: widget/trait.Widget.html#tymethod.lifecycle
/// [`register_child`]: #method.register_child
/// [`LifeCycleCtx::register_child`]: #method.register_child
/// [`LifeCycle::Register`]: enum.LifeCycle.html#variant.Register
pub struct LifeCycleCtx<'a> {
    pub(crate) command_queue: &'a mut CommandQueue,
    pub(crate) base_state: &'a mut BaseState,
    pub(crate) window_id: WindowId,
}

/// A mutable context provided to data update methods of widgets.
///
/// Widgets should call [`invalidate`] whenever a data change causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`invalidate`]: #method.invalidate
pub struct UpdateCtx<'a, 'b: 'a> {
    pub(crate) text_factory: &'a mut Text<'b>,
    pub(crate) window: &'a WindowHandle,
    // Discussion: we probably want to propagate more fine-grained
    // invalidations, which would mean a structure very much like
    // `EventCtx` (and possibly using the same structure). But for
    // now keep it super-simple.
    pub(crate) window_id: WindowId,
    pub(crate) base_state: &'a mut BaseState,
}

/// A context provided to layout handling methods of widgets.
///
/// As of now, the main service provided is access to a factory for
/// creating text layout objects, which are likely to be useful
/// during widget layout.
pub struct LayoutCtx<'a, 'b: 'a> {
    pub(crate) text_factory: &'a mut Text<'b>,
    pub(crate) window_id: WindowId,
}

/// Z-order paint operations with transformations.
pub struct ZOrderPaintOp {
    pub z_index: u32,
    pub paint_func: Box<dyn FnOnce(&mut PaintCtx) + 'static>,
    pub transform: Affine,
}

/// A context passed to paint methods of widgets.
///
/// Widgets paint their appearance by calling methods on the
/// `render_ctx`, which PaintCtx derefs to for convenience.
/// This struct is expected to grow, for example to include the
/// "damage region" indicating that only a subset of the entire
/// widget hierarchy needs repainting.
pub struct PaintCtx<'a, 'b: 'a> {
    /// The render context for actually painting.
    pub render_ctx: &'a mut Piet<'b>,
    pub window_id: WindowId,
    /// The z-order paint operations.
    pub z_ops: Vec<ZOrderPaintOp>,
    /// The currently visible region.
    pub(crate) region: Region,
    pub(crate) base_state: &'a BaseState,
    pub(crate) focus_widget: Option<WidgetId>,
}

/// A region of a widget, generally used to describe what needs to be drawn.
#[derive(Debug, Clone)]
pub struct Region(Rect);

impl<'a, 'b> EventCtx<'a, 'b> {
    /// Invalidate.
    ///
    /// Right now, it just invalidates the entire window, but we'll want
    /// finer grained invalidation before long.
    pub fn invalidate(&mut self) {
        // Note: for the current functionality, we could shortcut and just
        // request an invalidate on the window. But when we do fine-grained
        // invalidation, we'll want to compute the invalidation region, and
        // that needs to be propagated (with, likely, special handling for
        // scrolling).
        self.base_state.needs_inval = true;
    }

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    pub fn children_changed(&mut self) {
        self.base_state.children_changed = true;
    }

    /// Get an object which can create text layouts.
    pub fn text(&mut self) -> &mut Text<'b> {
        self.win_ctx.text_factory()
    }

    /// Set the cursor icon.
    ///
    /// Call this when handling a mouse move event, to set the cursor for the
    /// widget. A container widget can safely call this method, then recurse
    /// to its children, as a sequence of calls within an event propagation
    /// only has the effect of the last one (ie no need to worry about
    /// flashing).
    ///
    /// This method is expected to be called mostly from the [`MouseMoved`]
    /// event handler, but can also be called in response to other events,
    /// for example pressing a key to change the behavior of a widget.
    ///
    /// [`MouseMoved`]: enum.Event.html#variant.MouseDown
    pub fn set_cursor(&mut self, cursor: &Cursor) {
        *self.cursor = Some(cursor.clone());
    }

    /// Set the "active" state of the widget.
    ///
    /// See [`EventCtx::is_active`](struct.EventCtx.html#method.is_active).
    pub fn set_active(&mut self, active: bool) {
        self.base_state.is_active = active;
        // TODO: plumb mouse grab through to platform (through druid-shell)
    }

    /// The "hot" (aka hover) status of a widget.
    ///
    /// A widget is "hot" when the mouse is hovered over it. Widgets will
    /// often change their appearance as a visual indication that they
    /// will respond to mouse interaction.
    ///
    /// The hot status is computed from the widget's layout rect. In a
    /// container hierarchy, all widgets with layout rects containing the
    /// mouse position have hot status.
    ///
    /// Discussion: there is currently some confusion about whether a
    /// widget can be considered hot when some other widget is active (for
    /// example, when clicking to one widget and dragging to the next).
    /// The documentation should clearly state the resolution.
    pub fn is_hot(&self) -> bool {
        self.base_state.is_hot
    }

    /// The active status of a widget.
    ///
    /// Active status generally corresponds to a mouse button down. Widgets
    /// with behavior similar to a button will call [`set_active`] on mouse
    /// down and then up.
    ///
    /// When a widget is active, it gets mouse events even when the mouse
    /// is dragged away.
    ///
    /// [`set_active`]: struct.EventCtx.html#method.set_active
    pub fn is_active(&self) -> bool {
        self.base_state.is_active
    }

    /// Returns a reference to the current `WindowHandle`.
    ///
    /// Note: we're in the process of migrating towards providing functionality
    /// provided by the window handle in mutable contexts instead. If you're
    /// considering a new use of this method, try adding it to `WinCtx` and
    /// plumbing it through instead.
    pub fn window(&self) -> &WindowHandle {
        &self.window
    }

    /// Set the event as "handled", which stops its propagation to other
    /// widgets.
    pub fn set_handled(&mut self) {
        self.is_handled = true;
    }

    /// Determine whether the event has been handled by some other widget.
    pub fn is_handled(&self) -> bool {
        self.is_handled
    }

    /// The focus status of a widget.
    ///
    /// Focus means that the widget receives keyboard events.
    ///
    /// A widget can request focus using the [`request_focus`] method.
    /// This will generally result in a separate event propagation of
    /// a `FocusChanged` method, including sending `false` to the previous
    /// widget that held focus.
    ///
    /// Only one leaf widget at a time has focus. However, in a container
    /// hierarchy, all ancestors of that leaf widget are also invoked with
    /// `FocusChanged(true)`.
    ///
    /// Discussion question: is "is_focused" a better name?
    ///
    /// [`request_focus`]: struct.EventCtx.html#method.request_focus
    pub fn has_focus(&self) -> bool {
        let is_child = self
            .focus_widget
            .map(|id| self.base_state.children.contains(&id))
            .unwrap_or(false);
        is_child || self.focus_widget == Some(self.widget_id())
    }

    /// Request keyboard focus.
    ///
    /// See [`has_focus`] for more information.
    ///
    /// [`has_focus`]: struct.EventCtx.html#method.has_focus
    pub fn request_focus(&mut self) {
        self.base_state.request_focus = Some(FocusChange::Focus(self.widget_id()));
    }

    /// Transfer focus to the next focusable widget.
    ///
    /// This should only be called by a widget that currently has focus.
    pub fn focus_next(&mut self) {
        if self.focus_widget == Some(self.widget_id()) {
            self.base_state.request_focus = Some(FocusChange::Next);
        } else {
            log::warn!("focus_next can only be called by the currently focused widget");
        }
    }

    /// Transfer focus to the previous focusable widget.
    ///
    /// This should only be called by a widget that currently has focus.
    pub fn focus_prev(&mut self) {
        if self.focus_widget == Some(self.widget_id()) {
            self.base_state.request_focus = Some(FocusChange::Previous);
        } else {
            log::warn!("focus_prev can only be called by the currently focused widget");
        }
    }

    /// Give up focus.
    ///
    /// This should only be called by a widget that currently has focus.
    pub fn resign_focus(&mut self) {
        if self.focus_widget == Some(self.widget_id()) {
            self.base_state.request_focus = Some(FocusChange::Resign);
        } else {
            log::warn!("resign_focus can only be called by the currently focused widget");
        }
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        self.base_state.request_anim = true;
        self.base_state.needs_inval = true;
    }

    /// Request a timer event.
    ///
    /// The return value is a token, which can be used to associate the
    /// request with the event.
    pub fn request_timer(&mut self, deadline: Instant) -> TimerToken {
        self.base_state.request_timer = true;
        self.win_ctx.request_timer(deadline)
    }

    /// The layout size.
    ///
    /// This is the layout size as ultimately determined by the parent
    /// container, on the previous layout pass.
    ///
    /// Generally it will be the same as the size returned by the child widget's
    /// [`layout`] method.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    pub fn size(&self) -> Size {
        self.base_state.size()
    }

    /// Submit a [`Command`] to be run after this event is handled.
    ///
    /// Commands are run in the order they are submitted; all commands
    /// submitted during the handling of an event are executed before
    /// the [`update()`] method is called.
    ///
    /// [`Command`]: struct.Command.html
    /// [`update()`]: trait.Widget.html#tymethod.update
    pub fn submit_command(
        &mut self,
        command: impl Into<Command>,
        target: impl Into<Option<Target>>,
    ) {
        let target = target.into().unwrap_or_else(|| self.window_id.into());
        self.command_queue.push_back((target, command.into()))
    }

    /// Get the window id.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// get the `WidgetId` of the current widget.
    pub fn widget_id(&self) -> WidgetId {
        self.base_state.id
    }

    pub(crate) fn make_lifecycle_ctx(&mut self) -> LifeCycleCtx {
        LifeCycleCtx {
            command_queue: self.command_queue,
            base_state: self.base_state,
            window_id: self.window_id,
        }
    }
}

impl<'a> LifeCycleCtx<'a> {
    /// Invalidate.
    ///
    /// See [`EventCtx::invalidate`](struct.EventCtx.html#method.invalidate) for
    /// more discussion.
    pub fn invalidate(&mut self) {
        self.base_state.needs_inval = true;
    }

    /// Returns the current widget's `WidgetId`.
    pub fn widget_id(&self) -> WidgetId {
        self.base_state.id
    }

    /// Registers a child widget.
    ///
    /// This should only be called in response to a `LifeCycle::Register` event.
    ///
    /// In general, you should not need to call this method; it is handled by
    /// the `WidgetPod`.
    pub fn register_child(&mut self, child_id: WidgetId) {
        self.base_state.children.add(&child_id);
    }

    /// Register this widget to be eligile to accept focus automatically.
    pub fn register_for_focus(&mut self) {
        self.base_state.focus_chain.push(self.widget_id());
    }

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    pub fn children_changed(&mut self) {
        self.base_state.children_changed = true;
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        self.base_state.request_anim = true;
    }

    /// Submit a [`Command`] to be run after this event is handled.
    ///
    /// Commands are run in the order they are submitted; all commands
    /// submitted during the handling of an event are executed before
    /// the [`update()`] method is called.
    ///
    /// [`Command`]: struct.Command.html
    /// [`update()`]: trait.Widget.html#tymethod.update
    pub fn submit_command(
        &mut self,
        command: impl Into<Command>,
        target: impl Into<Option<Target>>,
    ) {
        let target = target.into().unwrap_or_else(|| self.window_id.into());
        self.command_queue.push_back((target, command.into()))
    }
}

impl<'a, 'b> UpdateCtx<'a, 'b> {
    /// Invalidate.
    ///
    /// See [`EventCtx::invalidate`](struct.EventCtx.html#method.invalidate) for
    /// more discussion.
    pub fn invalidate(&mut self) {
        self.base_state.needs_inval = true;
    }

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    pub fn children_changed(&mut self) {
        self.base_state.children_changed = true;
    }

    /// Get an object which can create text layouts.
    pub fn text(&mut self) -> &mut Text<'b> {
        self.text_factory
    }

    /// Returns a reference to the current `WindowHandle`.
    ///
    /// Note: For the most part we're trying to migrate `WindowHandle`
    /// functionality to `WinCtx`, but the update flow is the exception, as
    /// it's shared across multiple windows.
    //TODO: can we delete this? where is it used?
    pub fn window(&self) -> &WindowHandle {
        &self.window
    }

    /// Get the window id.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// get the `WidgetId` of the current widget.
    pub fn widget_id(&self) -> WidgetId {
        self.base_state.id
    }
}

impl<'a, 'b> LayoutCtx<'a, 'b> {
    /// Get an object which can create text layouts.
    pub fn text(&mut self) -> &mut Text<'b> {
        &mut self.text_factory
    }

    /// Get the window id.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }
}

impl<'a, 'b: 'a> PaintCtx<'a, 'b> {
    /// Query the "hot" state of the widget.
    ///
    /// See [`EventCtx::is_hot`](struct.EventCtx.html#method.is_hot) for
    /// additional information.
    pub fn is_hot(&self) -> bool {
        self.base_state.is_hot
    }

    /// Query the "active" state of the widget.
    ///
    /// See [`EventCtx::is_active`](struct.EventCtx.html#method.is_active) for
    /// additional information.
    pub fn is_active(&self) -> bool {
        self.base_state.is_active
    }

    /// Returns the layout size of the current widget.
    ///
    /// See [`EventCtx::size`](struct.EventCtx.html#method.size) for
    /// additional information.
    pub fn size(&self) -> Size {
        self.base_state.size()
    }

    /// Query the focus state of the widget.
    ///
    /// This is true only if this widget has focus.
    pub fn has_focus(&self) -> bool {
        self.focus_widget
            .map(|id| id == self.base_state.id)
            .unwrap_or(false)
    }

    /// Returns the currently visible [`Region`].
    ///
    /// [`Region`]: struct.Region.html
    #[inline]
    pub fn region(&self) -> &Region {
        &self.region
    }

    /// Creates a temporary `PaintCtx` with a new visible region, and calls
    /// the provided function with that `PaintCtx`.
    ///
    /// This is used by containers to ensure that their children have the correct
    /// visible region given their layout.
    pub fn with_child_ctx(&mut self, region: impl Into<Region>, f: impl FnOnce(&mut PaintCtx)) {
        let mut child_ctx = PaintCtx {
            render_ctx: self.render_ctx,
            base_state: self.base_state,
            z_ops: Vec::new(),
            window_id: self.window_id,
            focus_widget: self.focus_widget,
            region: region.into(),
        };
        f(&mut child_ctx);
        self.z_ops.append(&mut child_ctx.z_ops);
    }

    /// Allows to specify order for paint operations.
    ///
    /// Larger `z_idx` indicate that an operation will be executed later.
    pub fn paint_with_z_index(
        &mut self,
        z_index: u32,
        paint_func: impl FnOnce(&mut PaintCtx) + 'static,
    ) {
        let current_transform = self.render_ctx.current_transform();
        self.z_ops.push(ZOrderPaintOp {
            z_index,
            paint_func: Box::new(paint_func),
            transform: current_transform,
        })
    }
}

impl Region {
    /// Returns the smallest `Rect` that encloses the entire region.
    pub fn to_rect(&self) -> Rect {
        self.0
    }

    /// Returns `true` if `self` intersects with `other`.
    #[inline]
    pub fn intersects(&self, other: Rect) -> bool {
        self.0.intersect(other).area() > 0.
    }
}

impl From<Rect> for Region {
    fn from(src: Rect) -> Region {
        Region(src)
    }
}

impl<'a, 'b: 'a> Deref for PaintCtx<'a, 'b> {
    type Target = Piet<'b>;

    fn deref(&self) -> &Self::Target {
        self.render_ctx
    }
}

impl<'a, 'b: 'a> DerefMut for PaintCtx<'a, 'b> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.render_ctx
    }
}
