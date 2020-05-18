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

use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
    time::Duration,
};

use crate::core::{BaseState, CommandQueue, FocusChange};
use crate::piet::Piet;
use crate::piet::RenderContext;
use crate::{
    commands, Affine, Command, ContextMenu, Cursor, Insets, MenuDesc, Point, Rect, SingleUse, Size,
    Target, Text, TimerToken, Vec2, WidgetId, WindowDesc, WindowHandle, WindowId,
};

/// A mutable context provided to event handling methods of widgets.
///
/// Widgets should call [`request_paint`] whenever an event causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`request_paint`]: #method.request_paint
pub struct EventCtx<'a> {
    // Note: there's a bunch of state that's just passed down, might
    // want to group that into a single struct.
    pub(crate) cursor: &'a mut Option<Cursor>,
    /// Commands submitted to be run after this event.
    pub(crate) command_queue: &'a mut CommandQueue,
    pub(crate) window_id: WindowId,
    pub(crate) window: &'a WindowHandle,
    pub(crate) base_state: &'a mut BaseState,
    pub(crate) focus_widget: Option<WidgetId>,
    pub(crate) is_handled: bool,
    pub(crate) is_root: bool,
    pub(crate) app_data_type: TypeId,
}

/// A mutable context provided to the [`lifecycle`] method on widgets.
///
/// Certain methods on this context are only meaningful during the handling of
/// specific lifecycle events; for instance [`register_child`]
/// should only be called while handling [`LifeCycle::WidgetAdded`].
///
/// [`lifecycle`]: trait.Widget.html#tymethod.lifecycle
/// [`register_child`]: #method.register_child
/// [`LifeCycle::WidgetAdded`]: enum.LifeCycle.html#variant.WidgetAdded
pub struct LifeCycleCtx<'a> {
    pub(crate) command_queue: &'a mut CommandQueue,
    pub(crate) base_state: &'a mut BaseState,
    pub(crate) window_id: WindowId,
    pub(crate) window: &'a WindowHandle,
}

/// A mutable context provided to data update methods of widgets.
///
/// Widgets should call [`request_paint`] whenever a data change causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`request_paint`]: #method.request_paint
pub struct UpdateCtx<'a> {
    pub(crate) window: &'a WindowHandle,
    // Discussion: we probably want to propagate more fine-grained
    // invalidations, which would mean a structure very much like
    // `EventCtx` (and possibly using the same structure). But for
    // now keep it super-simple.
    pub(crate) command_queue: &'a mut CommandQueue,
    pub(crate) window_id: WindowId,
    pub(crate) base_state: &'a mut BaseState,
}

/// A context provided to layout handling methods of widgets.
///
/// As of now, the main service provided is access to a factory for
/// creating text layout objects, which are likely to be useful
/// during widget layout.
pub struct LayoutCtx<'a, 'b: 'a> {
    pub(crate) command_queue: &'a mut CommandQueue,
    pub(crate) base_state: &'a mut BaseState,
    pub(crate) text_factory: &'a mut Text<'b>,
    pub(crate) window_id: WindowId,
    pub(crate) window: &'a WindowHandle,
    pub(crate) mouse_pos: Option<Point>,
}

/// Z-order paint operations with transformations.
pub(crate) struct ZOrderPaintOp {
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
    pub(crate) z_ops: Vec<ZOrderPaintOp>,
    /// The currently visible region.
    pub(crate) region: Region,
    pub(crate) base_state: &'a BaseState,
    pub(crate) focus_widget: Option<WidgetId>,
    /// The approximate depth in the tree at the time of painting.
    pub(crate) depth: u32,
}

/// A region of a widget, generally used to describe what needs to be drawn.
///
/// This is currently just a single `Rect`, but may become more complicated in the future.  Although
/// this is just a wrapper around `Rect`, it has some different conventions. Mainly, "signed"
/// invalidation regions don't make sense. Therefore, a rectangle with non-positive width or height
/// is considered "empty", and all empty rectangles are treated the same.
#[derive(Debug, Clone)]
pub struct Region(Rect);

impl<'a> EventCtx<'a> {
    #[deprecated(since = "0.5.0", note = "use request_paint instead")]
    pub fn invalidate(&mut self) {
        self.request_paint();
    }

    /// Request a [`paint`] pass. This is equivalent to calling [`request_paint_rect`] for the
    /// widget's [`paint_rect`].
    ///
    /// [`paint`]: trait.Widget.html#tymethod.paint
    /// [`request_paint_rect`]: struct.EventCtx.html#method.request_paint_rect
    /// [`paint_rect`]: struct.WidgetPod.html#method.paint_rect
    pub fn request_paint(&mut self) {
        self.request_paint_rect(
            self.base_state.paint_rect() - self.base_state.layout_rect().origin().to_vec2(),
        );
    }

    /// Request a [`paint`] pass for redrawing a rectangle, which is given relative to our layout
    /// rectangle.
    ///
    /// [`paint`]: trait.Widget.html#tymethod.paint
    pub fn request_paint_rect(&mut self, rect: Rect) {
        self.base_state.invalid.add_rect(rect);
    }

    /// Request a layout pass.
    ///
    /// A Widget's [`layout`] method is always called when the widget tree
    /// changes, or the window is resized.
    ///
    /// If your widget would like to have layout called at any other time,
    /// (such as if it would like to change the layout of children in
    /// response to some event) it must call this method.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    pub fn request_layout(&mut self) {
        self.base_state.needs_layout = true;
    }

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    pub fn children_changed(&mut self) {
        self.base_state.children_changed = true;
        self.request_layout();
    }

    /// Get an object which can create text layouts.
    pub fn text(&mut self) -> Text {
        self.window.text()
    }

    /// Set the cursor icon.
    ///
    /// Call this when handling a mouse move event, to set the cursor for the
    /// widget. A container widget can safely call this method, then recurse
    /// to its children, as a sequence of calls within an event propagation
    /// only has the effect of the last one (ie no need to worry about
    /// flashing).
    ///
    /// This method is expected to be called mostly from the [`MouseMove`]
    /// event handler, but can also be called in response to other events,
    /// for example pressing a key to change the behavior of a widget.
    ///
    /// [`MouseMove`]: enum.Event.html#variant.MouseMove
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
    pub fn window(&self) -> &WindowHandle {
        &self.window
    }

    /// Create a new window.
    /// `T` must be the application's root `Data` type (the type provided to [`AppLauncher::launch`]).
    ///
    /// [`AppLauncher::launch`]: struct.AppLauncher.html#method.launch
    pub fn new_window<T: Any>(&mut self, desc: WindowDesc<T>) {
        if self.app_data_type == TypeId::of::<T>() {
            self.submit_command(
                Command::new(commands::NEW_WINDOW, SingleUse::new(desc)),
                Target::Global,
            );
        } else {
            const MSG: &str = "WindowDesc<T> - T must match the application data type.";
            if cfg!(debug_assertions) {
                panic!(MSG);
            } else {
                log::error!("EventCtx::new_window: {}", MSG)
            }
        }
    }

    /// Set the menu of the window containing the current widget.
    /// `T` must be the application's root `Data` type (the type provided to [`AppLauncher::launch`]).
    ///
    /// [`AppLauncher::launch`]: struct.AppLauncher.html#method.launch
    pub fn set_menu<T: Any>(&mut self, menu: MenuDesc<T>) {
        if self.app_data_type == TypeId::of::<T>() {
            self.submit_command(
                Command::new(commands::SET_MENU, menu),
                Target::Window(self.window_id),
            );
        } else {
            const MSG: &str = "MenuDesc<T> - T must match the application data type.";
            if cfg!(debug_assertions) {
                panic!(MSG);
            } else {
                log::error!("EventCtx::set_menu: {}", MSG)
            }
        }
    }

    /// Show the context menu in the window containing the current widget.
    /// `T` must be the application's root `Data` type (the type provided to [`AppLauncher::launch`]).
    ///
    /// [`AppLauncher::launch`]: struct.AppLauncher.html#method.launch
    pub fn show_context_menu<T: Any>(&mut self, menu: ContextMenu<T>) {
        if self.app_data_type == TypeId::of::<T>() {
            self.submit_command(
                Command::new(commands::SHOW_CONTEXT_MENU, menu),
                Target::Window(self.window_id),
            );
        } else {
            const MSG: &str = "ContextMenu<T> - T must match the application data type.";
            if cfg!(debug_assertions) {
                panic!(MSG);
            } else {
                log::error!("EventCtx::show_context_menu: {}", MSG)
            }
        }
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
    /// Returns `true` if this specific widget is focused.
    /// To check if any descendants are focused use [`has_focus`].
    ///
    /// Focus means that the widget receives keyboard events.
    ///
    /// A widget can request focus using the [`request_focus`] method.
    /// It's also possible to register for automatic focus via [`register_for_focus`].
    ///
    /// If a widget gains or loses focus it will get a [`LifeCycle::FocusChanged`] event.
    ///
    /// Only one widget at a time is focused. However due to the way events are routed,
    /// all ancestors of that widget will also receive keyboard events.
    ///
    /// [`request_focus`]: struct.EventCtx.html#method.request_focus
    /// [`register_for_focus`]: struct.LifeCycleCtx.html#method.register_for_focus
    /// [`LifeCycle::FocusChanged`]: enum.LifeCycle.html#variant.FocusChanged
    /// [`has_focus`]: struct.EventCtx.html#method.has_focus
    pub fn is_focused(&self) -> bool {
        self.focus_widget == Some(self.widget_id())
    }

    /// The (tree) focus status of a widget.
    ///
    /// Returns `true` if either this specific widget or any one of its descendants is focused.
    /// To check if only this specific widget is focused use [`is_focused`].
    ///
    /// See [`is_focused`] for more information about focus.
    ///
    /// [`is_focused`]: struct.EventCtx.html#method.is_focused
    pub fn has_focus(&self) -> bool {
        self.base_state.has_focus
    }

    /// Request keyboard focus.
    ///
    /// Calling this when the widget is already focused does nothing.
    ///
    /// See [`is_focused`] for more information about focus.
    ///
    /// [`is_focused`]: struct.EventCtx.html#method.is_focused
    pub fn request_focus(&mut self) {
        let id = self.widget_id();
        if self.focus_widget != Some(id) {
            self.base_state.request_focus = Some(FocusChange::Focus(id));
        }
    }

    /// Transfer focus to the next focusable widget.
    ///
    /// This should only be called by a widget that currently has focus.
    ///
    /// See [`is_focused`] for more information about focus.
    ///
    /// [`is_focused`]: struct.EventCtx.html#method.is_focused
    pub fn focus_next(&mut self) {
        if self.is_focused() {
            self.base_state.request_focus = Some(FocusChange::Next);
        } else {
            log::warn!("focus_next can only be called by the currently focused widget");
        }
    }

    /// Transfer focus to the previous focusable widget.
    ///
    /// This should only be called by a widget that currently has focus.
    ///
    /// See [`is_focused`] for more information about focus.
    ///
    /// [`is_focused`]: struct.EventCtx.html#method.is_focused
    pub fn focus_prev(&mut self) {
        if self.is_focused() {
            self.base_state.request_focus = Some(FocusChange::Previous);
        } else {
            log::warn!("focus_prev can only be called by the currently focused widget");
        }
    }

    /// Give up focus.
    ///
    /// This should only be called by a widget that currently has focus.
    ///
    /// See [`is_focused`] for more information about focus.
    ///
    /// [`is_focused`]: struct.EventCtx.html#method.is_focused
    pub fn resign_focus(&mut self) {
        if self.is_focused() {
            self.base_state.request_focus = Some(FocusChange::Resign);
        } else {
            log::warn!(
                "resign_focus can only be called by the currently focused widget ({:?})",
                self.widget_id()
            );
        }
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        self.base_state.request_anim = true;
        self.request_paint();
    }

    /// Request a timer event.
    ///
    /// The return value is a token, which can be used to associate the
    /// request with the event.
    pub fn request_timer(&mut self, deadline: Duration) -> TimerToken {
        self.base_state.request_timer = true;
        let timer_token = self.window.request_timer(deadline);
        self.base_state.add_timer(timer_token);
        timer_token
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
    /// the [`update`] method is called.
    ///
    /// [`Command`]: struct.Command.html
    /// [`update`]: trait.Widget.html#tymethod.update
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
}

impl<'a> LifeCycleCtx<'a> {
    #[deprecated(since = "0.5.0", note = "use request_paint instead")]
    pub fn invalidate(&mut self) {
        self.request_paint();
    }

    /// Request a [`paint`] pass. This is equivalent to calling [`request_paint_rect`] for the
    /// widget's [`paint_rect`].
    ///
    /// [`paint`]: trait.Widget.html#tymethod.paint
    /// [`request_paint_rect`]: struct.LifeCycleCtx.html#method.request_paint_rect
    /// [`paint_rect`]: struct.WidgetPod.html#method.paint_rect
    pub fn request_paint(&mut self) {
        self.request_paint_rect(
            self.base_state.paint_rect() - self.base_state.layout_rect().origin().to_vec2(),
        );
    }

    /// Request a [`paint`] pass for redrawing a rectangle, which is given relative to our layout
    /// rectangle.
    ///
    /// [`paint`]: trait.Widget.html#tymethod.paint
    pub fn request_paint_rect(&mut self, rect: Rect) {
        self.base_state.invalid.add_rect(rect);
    }

    /// Request layout.
    ///
    /// See [`EventCtx::request_layout`] for more information.
    ///
    /// [`EventCtx::request_layout`]: struct.EventCtx.html#method.request_layout
    pub fn request_layout(&mut self) {
        self.base_state.needs_layout = true;
    }

    /// Returns the current widget's `WidgetId`.
    pub fn widget_id(&self) -> WidgetId {
        self.base_state.id
    }

    /// Registers a child widget.
    ///
    /// This should only be called in response to a `LifeCycle::WidgetAdded` event.
    ///
    /// In general, you should not need to call this method; it is handled by
    /// the `WidgetPod`.
    pub fn register_child(&mut self, child_id: WidgetId) {
        self.base_state.children.add(&child_id);
    }

    /// Register this widget to be eligile to accept focus automatically.
    ///
    /// This should only be called in response to a [`LifeCycle::WidgetAdded`] event.
    ///
    /// See [`EventCtx::is_focused`] for more information about focus.
    ///
    /// [`LifeCycle::WidgetAdded`]: enum.Lifecycle.html#variant.WidgetAdded
    /// [`EventCtx::is_focused`]: struct.EventCtx.html#method.is_focused
    pub fn register_for_focus(&mut self) {
        self.base_state.focus_chain.push(self.widget_id());
    }

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    pub fn children_changed(&mut self) {
        self.base_state.children_changed = true;
        self.request_layout();
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        self.base_state.request_anim = true;
        self.request_paint();
    }

    /// Request a timer event.
    ///
    /// The return value is a token, which can be used to associate the
    /// request with the event.
    pub fn request_timer(&mut self, deadline: Duration) -> TimerToken {
        self.base_state.request_timer = true;
        let timer_token = self.window.request_timer(deadline);
        self.base_state.add_timer(timer_token);
        timer_token
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
    /// the [`update`] method is called.
    ///
    /// [`Command`]: struct.Command.html
    /// [`update`]: trait.Widget.html#tymethod.update
    pub fn submit_command(
        &mut self,
        command: impl Into<Command>,
        target: impl Into<Option<Target>>,
    ) {
        let target = target.into().unwrap_or_else(|| self.window_id.into());
        self.command_queue.push_back((target, command.into()))
    }
}

impl<'a> UpdateCtx<'a> {
    #[deprecated(since = "0.5.0", note = "use request_paint instead")]
    pub fn invalidate(&mut self) {
        self.request_paint();
    }

    /// Request a [`paint`] pass. This is equivalent to calling [`request_paint_rect`] for the
    /// widget's [`paint_rect`].
    ///
    /// [`paint`]: trait.Widget.html#tymethod.paint
    /// [`request_paint_rect`]: struct.UpdateCtx.html#method.request_paint_rect
    /// [`paint_rect`]: struct.WidgetPod.html#method.paint_rect
    pub fn request_paint(&mut self) {
        self.request_paint_rect(
            self.base_state.paint_rect() - self.base_state.layout_rect().origin().to_vec2(),
        );
    }

    /// Request a [`paint`] pass for redrawing a rectangle, which is given relative to our layout
    /// rectangle.
    ///
    /// [`paint`]: trait.Widget.html#tymethod.paint
    pub fn request_paint_rect(&mut self, rect: Rect) {
        self.base_state.invalid.add_rect(rect);
    }

    /// Request layout.
    ///
    /// See [`EventCtx::request_layout`] for more information.
    ///
    /// [`EventCtx::request_layout`]: struct.EventCtx.html#method.request_layout
    pub fn request_layout(&mut self) {
        self.base_state.needs_layout = true;
    }

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    pub fn children_changed(&mut self) {
        self.base_state.children_changed = true;
        self.request_layout();
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        self.base_state.request_anim = true;
        self.request_paint();
    }

    /// Request a timer event.
    ///
    /// The return value is a token, which can be used to associate the
    /// request with the event.
    pub fn request_timer(&mut self, deadline: Duration) -> TimerToken {
        self.base_state.request_timer = true;
        let timer_token = self.window.request_timer(deadline);
        self.base_state.add_timer(timer_token);
        timer_token
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

    /// Submit a [`Command`] to be run after layout and paint finish.
    ///
    /// **Note:**
    ///
    /// Commands submited during an `update` call are handled *after* update,
    /// layout, and paint have completed; this will trigger a new event cycle.
    ///
    /// [`Command`]: struct.Command.html
    pub fn submit_command(
        &mut self,
        command: impl Into<Command>,
        target: impl Into<Option<Target>>,
    ) {
        let target = target.into().unwrap_or_else(|| self.window_id.into());
        self.command_queue.push_back((target, command.into()))
    }

    /// Get an object which can create text layouts.
    pub fn text(&mut self) -> Text {
        self.window.text()
    }

    /// Returns a reference to the current `WindowHandle`.
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

    /// Set explicit paint [`Insets`] for this widget.
    ///
    /// You are not required to set explicit paint bounds unless you need
    /// to paint outside of your layout bounds. In this case, the argument
    /// should be an [`Insets`] struct that indicates where your widget
    /// needs to overpaint, relative to its bounds.
    ///
    /// For more information, see [`WidgetPod::paint_insets`].
    ///
    /// [`Insets`]: struct.Insets.html
    /// [`WidgetPod::paint_insets`]: struct.WidgetPod.html#method.paint_insets
    pub fn set_paint_insets(&mut self, insets: impl Into<Insets>) {
        self.base_state.paint_insets = insets.into().nonnegative();
    }
}

impl<'a, 'b: 'a> PaintCtx<'a, 'b> {
    /// get the `WidgetId` of the current widget.
    pub fn widget_id(&self) -> WidgetId {
        self.base_state.id
    }

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

    /// The focus status of a widget.
    ///
    /// Returns `true` if this specific widget is focused.
    /// To check if any descendants are focused use [`has_focus`].
    ///
    /// See [`EventCtx::is_focused`] for more information about focus.
    ///
    /// [`has_focus`]: #method.has_focus
    /// [`EventCtx::is_focused`]: struct.EventCtx.html#method.is_focused
    pub fn is_focused(&self) -> bool {
        self.focus_widget == Some(self.widget_id())
    }

    /// The (tree) focus status of a widget.
    ///
    /// Returns `true` if either this specific widget or any one of its descendants is focused.
    /// To check if only this specific widget is focused use [`is_focused`].
    ///
    /// See [`EventCtx::is_focused`] for more information about focus.
    ///
    /// [`is_focused`]: #method.is_focused
    /// [`EventCtx::is_focused`]: struct.EventCtx.html#method.is_focused
    pub fn has_focus(&self) -> bool {
        self.base_state.has_focus
    }

    /// The depth in the tree of the currently painting widget.
    ///
    /// This may be used in combination with [`paint_with_z_index`] in order
    /// to correctly order painting operations.
    ///
    /// The `depth` here may not be exact; it is only guaranteed that a child will
    /// have a greater depth than its parent.
    ///
    /// [`paint_with_z_index`]: #method.paint_with_z_index
    #[inline]
    pub fn depth(&self) -> u32 {
        self.depth
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
            depth: self.depth + 1,
        };
        f(&mut child_ctx);
        self.z_ops.append(&mut child_ctx.z_ops);
    }

    /// Saves the current context, executes the closures, and restores the context.
    ///
    /// This is useful if you would like to transform or clip or otherwise
    /// modify the drawing context but do not want that modification to
    /// effect other widgets.
    ///
    /// # Examples
    ///
    /// ```
    /// # use druid::{Env, PaintCtx, RenderContext, theme};
    /// # struct T;
    /// # impl T {
    /// fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
    ///     let clip_rect = ctx.size().to_rect().inset(5.0);
    ///     ctx.with_save(|ctx| {
    ///         ctx.clip(clip_rect);
    ///         ctx.stroke(clip_rect, &env.get(theme::PRIMARY_DARK), 5.0);
    ///     });
    /// }
    /// # }
    /// ```
    pub fn with_save(&mut self, f: impl FnOnce(&mut PaintCtx)) {
        if let Err(e) = self.render_ctx.save() {
            log::error!("Failed to save RenderContext: '{}'", e);
            return;
        }

        f(self);

        if let Err(e) = self.render_ctx.restore() {
            log::error!("Failed to restore RenderContext: '{}'", e);
        }
    }

    /// Allows to specify order for paint operations.
    ///
    /// Larger `z_index` indicate that an operation will be executed later.
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
    /// An empty region.
    pub const EMPTY: Region = Region(Rect::ZERO);

    /// Returns the smallest `Rect` that encloses the entire region.
    pub fn to_rect(&self) -> Rect {
        self.0
    }

    /// Returns `true` if `self` intersects with `other`.
    #[inline]
    pub fn intersects(&self, other: Rect) -> bool {
        self.0.intersect(other).area() > 0.
    }

    /// Returns `true` if this region is empty.
    pub fn is_empty(&self) -> bool {
        self.0.width() <= 0.0 || self.0.height() <= 0.0
    }

    /// Adds a new `Rect` to this region.
    ///
    /// This differs from `Rect::union` in its treatment of empty rectangles: an empty rectangle has
    /// no effect on the union.
    pub(crate) fn add_rect(&mut self, rect: Rect) {
        if self.is_empty() {
            self.0 = rect;
        } else if rect.width() > 0.0 && rect.height() > 0.0 {
            self.0 = self.0.union(rect);
        }
    }

    /// Modifies this region by including everything in the other region.
    pub(crate) fn merge_with(&mut self, other: Region) {
        self.add_rect(other.0);
    }

    /// Modifies this region by intersecting it with the given rectangle.
    pub(crate) fn intersect_with(&mut self, rect: Rect) {
        self.0 = self.0.intersect(rect);
    }
}

impl std::ops::AddAssign<Vec2> for Region {
    fn add_assign(&mut self, offset: Vec2) {
        self.0 = self.0 + offset;
    }
}

impl std::ops::SubAssign<Vec2> for Region {
    fn sub_assign(&mut self, offset: Vec2) {
        self.0 = self.0 - offset;
    }
}

impl From<Rect> for Region {
    fn from(src: Rect) -> Region {
        // We maintain the invariant that the width/height of the rect are non-negative.
        Region(src.abs())
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
