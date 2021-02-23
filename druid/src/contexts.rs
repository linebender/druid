// Copyright 2020 The Druid Authors.
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
    collections::VecDeque,
    ops::{Deref, DerefMut},
    time::Duration,
};

use crate::core::{CommandQueue, CursorChange, FocusChange, WidgetState};
use crate::env::KeyLike;
use crate::piet::{Piet, PietText, RenderContext};
use crate::shell::Region;
use crate::{
    commands, sub_window::SubWindowDesc, widget::Widget, Affine, Command, ContextMenu, Cursor,
    Data, Env, ExtEventSink, Insets, MenuDesc, Notification, Point, Rect, SingleUse, Size, Target,
    TimerToken, Vec2, WidgetId, WindowConfig, WindowDesc, WindowHandle, WindowId,
};

/// A macro for implementing methods on multiple contexts.
///
/// There are a lot of methods defined on multiple contexts; this lets us only
/// have to write them out once.
macro_rules! impl_context_method {
    ($ty:ty,  { $($method:item)+ } ) => {
        impl $ty { $($method)+ }
    };
    ( $ty:ty, $($more:ty),+, { $($method:item)+ } ) => {
        impl_context_method!($ty, { $($method)+ });
        impl_context_method!($($more),+, { $($method)+ });
    };
}

/// Static state that is shared between most contexts.
pub(crate) struct ContextState<'a> {
    pub(crate) command_queue: &'a mut CommandQueue,
    pub(crate) ext_handle: &'a ExtEventSink,
    pub(crate) window_id: WindowId,
    pub(crate) window: &'a WindowHandle,
    pub(crate) text: PietText,
    /// The id of the widget that currently has focus.
    pub(crate) focus_widget: Option<WidgetId>,
    pub(crate) root_app_data_type: TypeId,
}

/// A mutable context provided to event handling methods of widgets.
///
/// Widgets should call [`request_paint`] whenever an event causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`request_paint`]: #method.request_paint
pub struct EventCtx<'a, 'b> {
    pub(crate) state: &'a mut ContextState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) notifications: &'a mut VecDeque<Notification>,
    pub(crate) is_handled: bool,
    pub(crate) is_root: bool,
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
pub struct LifeCycleCtx<'a, 'b> {
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) state: &'a mut ContextState<'b>,
}

/// A mutable context provided to data update methods of widgets.
///
/// Widgets should call [`request_paint`] whenever a data change causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`request_paint`]: #method.request_paint
pub struct UpdateCtx<'a, 'b> {
    pub(crate) state: &'a mut ContextState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
    pub(crate) prev_env: Option<&'a Env>,
    pub(crate) env: &'a Env,
}

/// A context provided to layout handling methods of widgets.
///
/// As of now, the main service provided is access to a factory for
/// creating text layout objects, which are likely to be useful
/// during widget layout.
pub struct LayoutCtx<'a, 'b> {
    pub(crate) state: &'a mut ContextState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
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
/// In addition to the API below, [`PaintCtx`] derefs to an implemention of
/// the [`RenderContext`] trait, which defines the basic available drawing
/// commands.
pub struct PaintCtx<'a, 'b, 'c> {
    pub(crate) state: &'a mut ContextState<'b>,
    pub(crate) widget_state: &'a WidgetState,
    /// The render context for actually painting.
    pub render_ctx: &'a mut Piet<'c>,
    /// The z-order paint operations.
    pub(crate) z_ops: Vec<ZOrderPaintOp>,
    /// The currently visible region.
    pub(crate) region: Region,
    /// The approximate depth in the tree at the time of painting.
    pub(crate) depth: u32,
}

// methods on everyone
impl_context_method!(
    EventCtx<'_, '_>,
    UpdateCtx<'_, '_>,
    LifeCycleCtx<'_, '_>,
    PaintCtx<'_, '_, '_>,
    LayoutCtx<'_, '_>,
    {
        /// get the `WidgetId` of the current widget.
        pub fn widget_id(&self) -> WidgetId {
            self.widget_state.id
        }

        /// Returns a reference to the current `WindowHandle`.
        pub fn window(&self) -> &WindowHandle {
            &self.state.window
        }

        /// Get the `WindowId` of the current window.
        pub fn window_id(&self) -> WindowId {
            self.state.window_id
        }

        /// Get an object which can create text layouts.
        pub fn text(&mut self) -> &mut PietText {
            &mut self.state.text
        }
    }
);

// methods on everyone but layoutctx
impl_context_method!(
    EventCtx<'_, '_>,
    UpdateCtx<'_, '_>,
    LifeCycleCtx<'_, '_>,
    PaintCtx<'_, '_, '_>,
    {
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
            self.widget_state.size()
        }

        /// The origin of the widget in window coordinates, relative to the top left corner of the
        /// content area.
        pub fn window_origin(&self) -> Point {
            self.widget_state.window_origin()
        }

        /// Convert a point from the widget's coordinate space to the window's.
        ///
        /// The returned point is relative to the content area; it excludes window chrome.
        pub fn to_window(&self, widget_point: Point) -> Point {
            self.window_origin() + widget_point.to_vec2()
        }

        /// Convert a point from the widget's coordinate space to the screen's.
        /// See the [`Screen`] module
        ///
        /// [`Screen`]: crate::shell::Screen
        pub fn to_screen(&self, widget_point: Point) -> Point {
            let insets = self.window().content_insets();
            let content_origin = self.window().get_position() + Vec2::new(insets.x0, insets.y0);
            content_origin + self.to_window(widget_point).to_vec2()
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
            self.widget_state.is_hot
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
            self.widget_state.is_active
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
        /// [`has_focus`]: #method.has_focus
        pub fn is_focused(&self) -> bool {
            self.state.focus_widget == Some(self.widget_id())
        }

        /// The (tree) focus status of a widget.
        ///
        /// Returns `true` if either this specific widget or any one of its descendants is focused.
        /// To check if only this specific widget is focused use [`is_focused`],
        ///
        /// [`is_focused`]: #method.is_focused
        pub fn has_focus(&self) -> bool {
            self.widget_state.has_focus
        }
    }
);

impl_context_method!(EventCtx<'_, '_>, UpdateCtx<'_, '_>, {
    /// Set the cursor icon.
    ///
    /// This setting will be retained until [`clear_cursor`] is called, but it will only take
    /// effect when this widget is either [`hot`] or [`active`]. If a child widget also sets a
    /// cursor, the child widget's cursor will take precedence. (If that isn't what you want, use
    /// [`override_cursor`] instead.)
    ///
    /// [`clear_cursor`]: EventCtx::clear_cursor
    /// [`override_cursor`]: EventCtx::override_cursor
    /// [`hot`]: EventCtx::is_hot
    /// [`active`]: EventCtx::is_active
    pub fn set_cursor(&mut self, cursor: &Cursor) {
        self.widget_state.cursor_change = CursorChange::Set(cursor.clone());
    }

    /// Override the cursor icon.
    ///
    /// This setting will be retained until [`clear_cursor`] is called, but it will only take
    /// effect when this widget is either [`hot`] or [`active`]. This will override the cursor
    /// preferences of a child widget. (If that isn't what you want, use [`set_cursor`] instead.)
    ///
    /// [`clear_cursor`]: EventCtx::clear_cursor
    /// [`set_cursor`]: EventCtx::override_cursor
    /// [`hot`]: EventCtx::is_hot
    /// [`active`]: EventCtx::is_active
    pub fn override_cursor(&mut self, cursor: &Cursor) {
        self.widget_state.cursor_change = CursorChange::Override(cursor.clone());
    }

    /// Clear the cursor icon.
    ///
    /// This undoes the effect of [`set_cursor`] and [`override_cursor`].
    ///
    /// [`override_cursor`]: EventCtx::override_cursor
    /// [`set_cursor`]: EventCtx::set_cursor
    pub fn clear_cursor(&mut self) {
        self.widget_state.cursor_change = CursorChange::Default;
    }
});

// methods on event, update, and lifecycle
impl_context_method!(EventCtx<'_, '_>, UpdateCtx<'_, '_>, LifeCycleCtx<'_, '_>, {
    /// Request a [`paint`] pass. This is equivalent to calling
    /// [`request_paint_rect`] for the widget's [`paint_rect`].
    ///
    /// [`paint`]: trait.Widget.html#tymethod.paint
    /// [`request_paint_rect`]: #method.request_paint_rect
    /// [`paint_rect`]: struct.WidgetPod.html#method.paint_rect
    pub fn request_paint(&mut self) {
        self.widget_state.invalid.set_rect(
            self.widget_state.paint_rect() - self.widget_state.layout_rect().origin().to_vec2(),
        );
    }

    /// Request a [`paint`] pass for redrawing a rectangle, which is given
    /// relative to our layout rectangle.
    ///
    /// [`paint`]: trait.Widget.html#tymethod.paint
    pub fn request_paint_rect(&mut self, rect: Rect) {
        self.widget_state.invalid.add_rect(rect);
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
        self.widget_state.needs_layout = true;
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        self.widget_state.request_anim = true;
    }

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    pub fn children_changed(&mut self) {
        self.widget_state.children_changed = true;
        self.request_layout();
    }

    /// Set the menu of the window containing the current widget.
    /// `T` must be the application's root `Data` type (the type provided to [`AppLauncher::launch`]).
    ///
    /// [`AppLauncher::launch`]: struct.AppLauncher.html#method.launch
    pub fn set_menu<T: Any>(&mut self, menu: MenuDesc<T>) {
        self.state.set_menu(menu);
    }

    /// Create a new sub-window.
    ///
    /// The sub-window will have its app data synchronised with caller's nearest ancestor [`WidgetPod`].
    /// 'U' must be the type of the nearest surrounding [`WidgetPod`]. The 'data' argument should be
    /// the current value of data  for that widget.
    ///
    /// [`WidgetPod`]: struct.WidgetPod.html
    // TODO - dynamically check that the type of the pod we are registering this on is the same as the type of the
    // requirement. Needs type ids recorded. This goes wrong if you don't have a pod between you and a lens.
    pub fn new_sub_window<W: Widget<U> + 'static, U: Data>(
        &mut self,
        window_config: WindowConfig,
        widget: W,
        data: U,
        env: Env,
    ) -> WindowId {
        let req = SubWindowDesc::new(self.widget_id(), window_config, widget, data, env);
        let window_id = req.window_id;
        self.widget_state
            .add_sub_window_host(window_id, req.host_id);
        self.submit_command(commands::NEW_SUB_WINDOW.with(SingleUse::new(req)));
        window_id
    }
});

// methods on everyone but paintctx
impl_context_method!(
    EventCtx<'_, '_>,
    UpdateCtx<'_, '_>,
    LifeCycleCtx<'_, '_>,
    LayoutCtx<'_, '_>,
    {
        /// Submit a [`Command`] to be run after this event is handled.
        ///
        /// Commands are run in the order they are submitted; all commands
        /// submitted during the handling of an event are executed before
        /// the [`update`] method is called; events submitted during [`update`]
        /// are handled after painting.
        ///
        /// [`Target::Auto`] commands will be sent to the window containing the widget.
        ///
        /// [`Command`]: struct.Command.html
        /// [`update`]: trait.Widget.html#tymethod.update
        pub fn submit_command(&mut self, cmd: impl Into<Command>) {
            self.state.submit_command(cmd.into())
        }

        /// Returns an [`ExtEventSink`] that can be moved between threads,
        /// and can be used to submit commands back to the application.
        ///
        /// [`ExtEventSink`]: struct.ExtEventSink.html
        pub fn get_external_handle(&self) -> ExtEventSink {
            self.state.ext_handle.clone()
        }

        /// Request a timer event.
        ///
        /// The return value is a token, which can be used to associate the
        /// request with the event.
        pub fn request_timer(&mut self, deadline: Duration) -> TimerToken {
            self.state.request_timer(&mut self.widget_state, deadline)
        }
    }
);

impl EventCtx<'_, '_> {
    /// Submit a [`Notification`].
    ///
    /// The provided argument can be a [`Selector`] or a [`Command`]; this lets
    /// us work with the existing API for addding a payload to a [`Selector`].
    ///
    /// If the argument is a `Command`, the command's target will be ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// # use druid::{Event, EventCtx, Selector};
    /// const IMPORTANT_EVENT: Selector<String> = Selector::new("druid-example.important-event");
    ///
    /// fn check_event(ctx: &mut EventCtx, event: &Event) {
    ///     if is_this_the_event_we_were_looking_for(event) {
    ///         ctx.submit_notification(IMPORTANT_EVENT.with("That's the one".to_string()))
    ///     }
    /// }
    ///
    /// # fn is_this_the_event_we_were_looking_for(event: &Event) -> bool { true }
    /// ```
    ///
    /// [`Selector`]: crate::Selector
    pub fn submit_notification(&mut self, note: impl Into<Command>) {
        let note = note.into().into_notification(self.widget_state.id);
        self.notifications.push_back(note);
    }

    /// Set the "active" state of the widget.
    ///
    /// See [`EventCtx::is_active`](struct.EventCtx.html#method.is_active).
    pub fn set_active(&mut self, active: bool) {
        self.widget_state.is_active = active;
        // TODO: plumb mouse grab through to platform (through druid-shell)
    }

    /// Create a new window.
    /// `T` must be the application's root `Data` type (the type provided to [`AppLauncher::launch`]).
    ///
    /// [`AppLauncher::launch`]: struct.AppLauncher.html#method.launch
    pub fn new_window<T: Any>(&mut self, desc: WindowDesc<T>) {
        if self.state.root_app_data_type == TypeId::of::<T>() {
            self.submit_command(
                commands::NEW_WINDOW
                    .with(SingleUse::new(Box::new(desc)))
                    .to(Target::Global),
            );
        } else {
            debug_panic!("EventCtx::new_window<T> - T must match the application data type.");
        }
    }

    /// Show the context menu in the window containing the current widget.
    /// `T` must be the application's root `Data` type (the type provided to [`AppLauncher::launch`]).
    ///
    /// [`AppLauncher::launch`]: struct.AppLauncher.html#method.launch
    pub fn show_context_menu<T: Any>(&mut self, menu: ContextMenu<T>) {
        if self.state.root_app_data_type == TypeId::of::<T>() {
            self.submit_command(
                commands::SHOW_CONTEXT_MENU
                    .with(Box::new(menu))
                    .to(Target::Window(self.state.window_id)),
            );
        } else {
            debug_panic!(
                "EventCtx::show_context_menu<T> - T must match the application data type."
            );
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

    /// Request keyboard focus.
    ///
    /// Because only one widget can be focused at a time, multiple focus requests
    /// from different widgets during a single event cycle means that the last
    /// widget that requests focus will override the previous requests.
    ///
    /// See [`is_focused`] for more information about focus.
    ///
    /// [`is_focused`]: struct.EventCtx.html#method.is_focused
    pub fn request_focus(&mut self) {
        // We need to send the request even if we're currently focused,
        // because we may have a sibling widget that already requested focus
        // and we have no way of knowing that yet. We need to override that
        // to deliver on the "last focus request wins" promise.
        let id = self.widget_id();
        self.widget_state.request_focus = Some(FocusChange::Focus(id));
    }

    /// Transfer focus to the widget with the given `WidgetId`.
    ///
    /// See [`is_focused`] for more information about focus.
    ///
    /// [`is_focused`]: struct.EventCtx.html#method.is_focused
    pub fn set_focus(&mut self, target: WidgetId) {
        self.widget_state.request_focus = Some(FocusChange::Focus(target));
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
            self.widget_state.request_focus = Some(FocusChange::Next);
        } else {
            tracing::warn!("focus_next can only be called by the currently focused widget");
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
            self.widget_state.request_focus = Some(FocusChange::Previous);
        } else {
            tracing::warn!("focus_prev can only be called by the currently focused widget");
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
            self.widget_state.request_focus = Some(FocusChange::Resign);
        } else {
            tracing::warn!(
                "resign_focus can only be called by the currently focused widget ({:?})",
                self.widget_id()
            );
        }
    }

    /// Request an update cycle.
    ///
    /// After this, `update` will be called on the widget in the next update cycle, even
    /// if there's not a data change.
    ///
    /// The use case for this method is when a container widget synthesizes data for its
    /// children. This is appropriate in specialized cases, but before reaching for this
    /// method, consider whether it might be better to refactor to be more idiomatic, in
    /// particular to make that data available in the app state.
    pub fn request_update(&mut self) {
        self.widget_state.request_update = true;
    }
}

impl UpdateCtx<'_, '_> {
    /// Returns `true` if this widget or a descendent as explicitly requested
    /// an update call.
    ///
    /// This should only be needed in advanced cases;
    /// see [`EventCtx::request_update`] for more information.
    ///
    /// [`EventCtx::request_update`]: struct.EventCtx.html#method.request_update
    pub fn has_requested_update(&mut self) -> bool {
        self.widget_state.request_update
    }

    /// Returns `true` if the current [`Env`] has changed since the previous
    /// [`update`] call.
    ///
    /// [`Env`]: struct.Env.html
    /// [`update`]: trait.Widget.html#tymethod.update
    pub fn env_changed(&self) -> bool {
        self.prev_env.is_some()
    }

    /// Returns `true` if the given key has changed since the last [`update`]
    /// call.
    ///
    /// The argument can be anything that is resolveable from the [`Env`],
    /// such as a [`Key`] or a [`KeyOrValue`].
    ///
    /// [`update`]: trait.Widget.html#tymethod.update
    /// [`Env`]: struct.Env.html
    /// [`Key`]: struct.Key.html
    /// [`KeyOrValue`]: enum.KeyOrValue.html
    pub fn env_key_changed<T>(&self, key: &impl KeyLike<T>) -> bool {
        match self.prev_env.as_ref() {
            Some(prev) => key.changed(prev, self.env),
            None => false,
        }
    }
}

impl LifeCycleCtx<'_, '_> {
    /// Registers a child widget.
    ///
    /// This should only be called in response to a `LifeCycle::WidgetAdded` event.
    ///
    /// In general, you should not need to call this method; it is handled by
    /// the `WidgetPod`.
    pub fn register_child(&mut self, child_id: WidgetId) {
        self.widget_state.children.add(&child_id);
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
        self.widget_state.focus_chain.push(self.widget_id());
    }
}

impl LayoutCtx<'_, '_> {
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
        self.widget_state.paint_insets = insets.into().nonnegative();
    }

    /// Set an explicit baseline position for this widget.
    ///
    /// The baseline position is used to align widgets that contain text,
    /// such as buttons, labels, and other controls. It may also be used
    /// by other widgets that are opinionated about how they are aligned
    /// relative to neighbouring text, such as switches or checkboxes.
    ///
    /// The provided value should be the distance from the *bottom* of the
    /// widget to the baseline.
    pub fn set_baseline_offset(&mut self, baseline: f64) {
        self.widget_state.baseline_offset = baseline
    }
}

impl PaintCtx<'_, '_, '_> {
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

    /// Returns the region that needs to be repainted.
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
            state: self.state,
            widget_state: self.widget_state,
            z_ops: Vec::new(),
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
            tracing::error!("Failed to save RenderContext: '{}'", e);
            return;
        }

        f(self);

        if let Err(e) = self.render_ctx.restore() {
            tracing::error!("Failed to restore RenderContext: '{}'", e);
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

impl<'a> ContextState<'a> {
    pub(crate) fn new<T: 'static>(
        command_queue: &'a mut CommandQueue,
        ext_handle: &'a ExtEventSink,
        window: &'a WindowHandle,
        window_id: WindowId,
        focus_widget: Option<WidgetId>,
    ) -> Self {
        ContextState {
            command_queue,
            ext_handle,
            window,
            window_id,
            focus_widget,
            text: window.text(),
            root_app_data_type: TypeId::of::<T>(),
        }
    }

    fn submit_command(&mut self, command: Command) {
        self.command_queue
            .push_back(command.default_to(self.window_id.into()));
    }

    fn set_menu<T: Any>(&mut self, menu: MenuDesc<T>) {
        if self.root_app_data_type == TypeId::of::<T>() {
            self.submit_command(
                commands::SET_MENU
                    .with(Box::new(menu))
                    .to(Target::Window(self.window_id)),
            );
        } else {
            debug_panic!("EventCtx::set_menu<T> - T must match the application data type.");
        }
    }

    fn request_timer(&self, widget_state: &mut WidgetState, deadline: Duration) -> TimerToken {
        let timer_token = self.window.request_timer(deadline);
        widget_state.add_timer(timer_token);
        timer_token
    }
}

impl<'c> Deref for PaintCtx<'_, '_, 'c> {
    type Target = Piet<'c>;

    fn deref(&self) -> &Self::Target {
        self.render_ctx
    }
}

impl<'c> DerefMut for PaintCtx<'_, '_, 'c> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.render_ctx
    }
}
