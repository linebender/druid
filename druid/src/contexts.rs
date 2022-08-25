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
    collections::{HashMap, VecDeque},
    ops::{Deref, DerefMut},
    rc::Rc,
    time::Duration,
};
use tracing::{error, trace, warn};

use crate::commands::SCROLL_TO_VIEW;
use crate::core::{CommandQueue, CursorChange, FocusChange, WidgetState};
use crate::env::KeyLike;
use crate::menu::ContextMenu;
use crate::piet::{Piet, PietText, RenderContext};
use crate::shell::text::Event as ImeInvalidation;
use crate::shell::Region;
use crate::text::{ImeHandlerRef, TextFieldRegistration};
use crate::{
    commands, sub_window::SubWindowDesc, widget::Widget, Affine, Command, Cursor, Data, Env,
    ExtEventSink, Insets, Menu, Notification, Point, Rect, SingleUse, Size, Target, TimerToken,
    Vec2, WidgetId, WindowConfig, WindowDesc, WindowHandle, WindowId,
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

/// A macro for implementing context traits for multiple contexts.
macro_rules! impl_context_trait{
    ($tr:ty => $ty:ty,  { $($method:item)+ } ) => {
        impl<'b> $tr for $ty { $($method)+ }
    };
    ($tr:ty => $ty:ty, $($more:ty),+, { $($method:item)+ } ) => {
        impl_context_trait!($tr => $ty, { $($method)+ });
        impl_context_trait!($tr => $($more),+, { $($method)+ });
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
    pub(crate) timers: &'a mut HashMap<TimerToken, WidgetId>,
    pub(crate) text_registrations: &'a mut Vec<TextFieldRegistration>,
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
    pub(crate) state: &'a mut ContextState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
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
}

/// Z-order paint operations with transformations.
pub(crate) struct ZOrderPaintOp {
    pub z_index: u32,
    pub paint_func: Box<dyn FnOnce(&mut PaintCtx) + 'static>,
    pub transform: Affine,
}

/// A context passed to paint methods of widgets.
///
/// In addition to the API below, [`PaintCtx`] derefs to an implementation of
/// the [`RenderContext`] trait, which defines the basic available drawing
/// commands.
pub struct PaintCtx<'a, 'b, 'c> {
    pub(crate) state: &'a mut ContextState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
    /// The render context for actually painting.
    pub render_ctx: &'a mut Piet<'c>,
    /// The z-order paint operations.
    pub(crate) z_ops: Vec<ZOrderPaintOp>,
    /// The currently visible region.
    pub(crate) region: Region,
    /// The approximate depth in the tree at the time of painting.
    pub(crate) depth: u32,
}

/// The state of a widget and its global context.
pub struct State<'a, 'b> {
    #[allow(dead_code)]
    pub(crate) state: &'a mut ContextState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

/// trait for accessing state and widget_state of the context.
pub trait AnyCtx<'b> {
    /// Get the state of a widget.
    ///
    /// This method can be used to perform merge_up of the widget state with a generic ctx
    /// This method is intended to be used only by the framework.
    fn state<'a>(&'a mut self) -> State<'a, 'b>;
}

/// Convenience trait for code generic over contexts.
///
/// Methods to do with commands and timers.
/// Available to all contexts but PaintCtx.
pub trait CommandCtx<'b>: AnyCtx<'b> {
    /// Submit a [`Command`] to be run after this event is handled. See [`submit_command`].
    ///
    /// [`submit_command`]: EventCtx::submit_command
    fn submit_command(&mut self, cmd: impl Into<Command>);
    /// Returns an [`ExtEventSink`] for submitting commands from other threads. See ['get_external_handle'].
    ///
    /// [`get_external_handle`]: EventCtx::get_external_handle
    fn get_external_handle(&self) -> ExtEventSink;
    /// Request a timer event. See [`request_timer`]
    ///
    /// [`request_timer`]: EventCtx::request_timer
    fn request_timer(&mut self, deadline: Duration) -> TimerToken;
}

/// Convenience trait for invalidation and request methods available on multiple contexts.
///
/// These methods are available on [`EventCtx`], [`LifeCycleCtx`], and [`UpdateCtx`].
pub trait RequestCtx<'b>: CommandCtx<'b> {
    /// Request a [`paint`] pass. See ['request_paint']
    ///
    /// ['request_paint']: EventCtx::request_paint
    fn request_paint(&mut self);
    /// Request a [`paint`] pass for redrawing a rectangle. See [`request_paint_rect`].
    ///
    /// [`request_paint_rect`]: EventCtx::request_paint_rect
    /// [`paint`]: Widget::paint
    fn request_paint_rect(&mut self, rect: Rect);
    /// Request a layout pass. See [`request_layout`].
    ///
    /// [`request_layout`]: EventCtx::request_layout
    fn request_layout(&mut self);
    /// Request an animation frame. See [`request_anim_frame`].
    ///
    /// [`request_anim_frame`]: EventCtx::request_anim_frame
    fn request_anim_frame(&mut self);
    /// Indicate that your children have changed. See [`children_changed`].
    ///
    /// [`children_changed`]: EventCtx::children_changed
    fn children_changed(&mut self);
    /// Create a new sub-window. See [`new_sub_window`].
    ///
    /// [`new_sub_window`]: EventCtx::new_sub_window
    fn new_sub_window<W: Widget<U> + 'static, U: Data>(
        &mut self,
        window_config: WindowConfig,
        widget: W,
        data: U,
        env: Env,
    ) -> WindowId;
    /// Change the disabled state of this widget. See [`set_disabled`].
    ///
    /// [`set_disabled`]: EventCtx::set_disabled
    fn set_disabled(&mut self, disabled: bool);
    /// Indicate that text input state has changed. See [`invalidate_text_input`].
    ///
    /// [`invalidate_text_input`]: EventCtx::invalidate_text_input
    fn invalidate_text_input(&mut self, event: ImeInvalidation);
    /// Scrolls this widget into view.
    ///
    /// [`scroll_to_view`]: EventCtx::scroll_to_view
    fn scroll_to_view(&mut self);
    /// Scrolls the area into view. See [`scroll_area_to_view`].
    ///
    /// [`scroll_area_to_view`]: EventCtx::scroll_area_to_view
    fn scroll_area_to_view(&mut self, area: Rect);
}

impl_context_trait!(
    AnyCtx<'b> => EventCtx<'_, 'b>, UpdateCtx<'_, 'b>, LifeCycleCtx<'_, 'b>, LayoutCtx<'_, 'b>, PaintCtx<'_, 'b, '_>,
    {
        fn state<'a>(&'a mut self) -> State<'a, 'b> {
            State {
                state: &mut *self.state,
                widget_state: &mut *self.widget_state,
            }
        }
    }
);

impl_context_trait!(
    CommandCtx<'b> => EventCtx<'_, 'b>, UpdateCtx<'_, 'b>, LifeCycleCtx<'_, 'b>, LayoutCtx<'_, 'b>,
    {

        fn submit_command(&mut self, cmd: impl Into<Command>) {
            Self::submit_command(self, cmd)
        }

        fn get_external_handle(&self) -> ExtEventSink {
            Self::get_external_handle(self)
        }

        fn request_timer(&mut self, deadline: Duration) -> TimerToken {
            Self::request_timer(self, deadline)
        }
    }
);

impl_context_trait!(
    RequestCtx<'b> => EventCtx<'_, 'b>, UpdateCtx<'_, 'b>, LifeCycleCtx<'_, 'b>,
    {
        fn request_paint(&mut self) {
            Self::request_paint(self)
        }

        fn request_paint_rect(&mut self, rect: Rect) {
            Self::request_paint_rect(self, rect)
        }

        fn request_layout(&mut self) {
            Self::request_layout(self)
        }

        fn request_anim_frame(&mut self) {
            Self::request_anim_frame(self)
        }

        fn children_changed(&mut self) {
            Self::children_changed(self)
        }

        fn new_sub_window<W: Widget<U> + 'static, U: Data>(
            &mut self,
            window_config: WindowConfig,
            widget: W,
            data: U,
            env: Env,
        ) -> WindowId {
            Self::new_sub_window(self, window_config, widget, data, env)
        }

        fn set_disabled(&mut self, disabled: bool) {
            Self::set_disabled(self, disabled)
        }

        fn invalidate_text_input(&mut self, event: ImeInvalidation) {
            Self::invalidate_text_input(self, event)
        }

        fn scroll_to_view(&mut self) {
            Self::scroll_to_view(self)
        }

        fn scroll_area_to_view(&mut self, area: Rect) {
            Self::scroll_area_to_view(self, area)
        }
    }
);

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
            self.state.window
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

        /// The disabled state of a widget.
        ///
        /// Returns `true` if this widget or any of its ancestors is explicitly disabled.
        /// To make this widget explicitly disabled use [`set_disabled`].
        ///
        /// Disabled means that this widget should not change the state of the application. What
        /// that means is not entirely clear but in any it should not change its data. Therefore
        /// others can use this as a safety mechanism to prevent the application from entering an
        /// illegal state.
        /// For an example the decrease button of a counter of type `usize` should be disabled if the
        /// value is `0`.
        ///
        /// [`set_disabled`]: EventCtx::set_disabled
        pub fn is_disabled(&self) -> bool {
            self.widget_state.is_disabled()
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
        trace!("set_cursor {:?}", cursor);
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
        trace!("override_cursor {:?}", cursor);
        self.widget_state.cursor_change = CursorChange::Override(cursor.clone());
    }

    /// Clear the cursor icon.
    ///
    /// This undoes the effect of [`set_cursor`] and [`override_cursor`].
    ///
    /// [`override_cursor`]: EventCtx::override_cursor
    /// [`set_cursor`]: EventCtx::set_cursor
    pub fn clear_cursor(&mut self) {
        trace!("clear_cursor");
        self.widget_state.cursor_change = CursorChange::Default;
    }
});

//methods on event, update and layout.
impl_context_method!(EventCtx<'_, '_>, UpdateCtx<'_, '_>, LayoutCtx<'_, '_>, {
    /// Indicate that your view_context has changed.
    ///
    /// Widgets must call this method after changing the clip region of thier children.
    /// The other parts of view_context (cursor_position and global origin) are tracked internally.
    pub fn view_context_changed(&mut self) {
        self.widget_state.view_context_changed = true;
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
        trace!("request_paint");
        self.widget_state.invalid.set_rect(
            self.widget_state.paint_rect() - self.widget_state.layout_rect().origin().to_vec2(),
        );
    }

    /// Request a [`paint`] pass for redrawing a rectangle, which is given
    /// relative to our layout rectangle.
    ///
    /// [`paint`]: trait.Widget.html#tymethod.paint
    pub fn request_paint_rect(&mut self, rect: Rect) {
        trace!("request_paint_rect {}", rect);
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
        trace!("request_layout");
        self.widget_state.needs_layout = true;
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        trace!("request_anim_frame");
        self.widget_state.request_anim = true;
    }

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child, removing a child or changing which
    /// children are hidden (see [`should_propagate_to_hidden`]).
    ///
    /// [`should_propagate_to_hidden`]: crate::Event::should_propagate_to_hidden
    pub fn children_changed(&mut self) {
        trace!("children_changed");
        self.widget_state.children_changed = true;
        self.widget_state.update_focus_chain = true;
        self.request_layout();
    }

    /// Set the disabled state for this widget.
    ///
    /// Setting this to `false` does not mean a widget is not still disabled; for instance it may
    /// still be disabled by an ancestor. See [`is_disabled`] for more information.
    ///
    /// Calling this method during [`LifeCycle::DisabledChanged`] has no effect.
    ///
    /// [`LifeCycle::DisabledChanged`]: struct.LifeCycle.html#variant.DisabledChanged
    /// [`is_disabled`]: EventCtx::is_disabled
    pub fn set_disabled(&mut self, disabled: bool) {
        // widget_state.children_disabled_changed is not set because we want to be able to delete
        // changes that happened during DisabledChanged.
        self.widget_state.is_explicitly_disabled_new = disabled;
    }

    /// Indicate that text input state has changed.
    ///
    /// A widget that accepts text input should call this anytime input state
    /// (such as the text or the selection) changes as a result of a non text-input
    /// event.
    pub fn invalidate_text_input(&mut self, event: ImeInvalidation) {
        let payload = commands::ImeInvalidation {
            widget: self.widget_id(),
            event,
        };
        let cmd = commands::INVALIDATE_IME
            .with(payload)
            .to(Target::Window(self.window_id()));
        self.submit_command(cmd);
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
        trace!("new_sub_window");
        let req = SubWindowDesc::new(self.widget_id(), window_config, widget, data, env);
        let window_id = req.window_id;
        self.widget_state
            .add_sub_window_host(window_id, req.host_id);
        self.submit_command(commands::NEW_SUB_WINDOW.with(SingleUse::new(req)));
        window_id
    }

    /// Scrolls this widget into view.
    ///
    /// If this widget is only partially visible or not visible at all because of [`Scroll`]s
    /// it is wrapped in, they will do the minimum amount of scrolling necessary to bring this
    /// widget fully into view.
    ///
    /// If the widget is [`hidden`], this method has no effect.
    ///
    /// This functionality is achieved by sending a [`SCROLL_TO_VIEW`] notification.
    ///
    /// [`Scroll`]: crate::widget::Scroll
    /// [`hidden`]: crate::Event::should_propagate_to_hidden
    /// [`SCROLL_TO_VIEW`]: crate::commands::SCROLL_TO_VIEW
    pub fn scroll_to_view(&mut self) {
        self.scroll_area_to_view(self.size().to_rect())
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
            trace!("submit_command");
            self.state.submit_command(cmd.into())
        }

        /// Returns an [`ExtEventSink`] that can be moved between threads,
        /// and can be used to submit commands back to the application.
        ///
        /// [`ExtEventSink`]: struct.ExtEventSink.html
        pub fn get_external_handle(&self) -> ExtEventSink {
            trace!("get_external_handle");
            self.state.ext_handle.clone()
        }

        /// Request a timer event.
        ///
        /// The return value is a token, which can be used to associate the
        /// request with the event.
        pub fn request_timer(&mut self, deadline: Duration) -> TimerToken {
            trace!("request_timer deadline={:?}", deadline);
            self.state.request_timer(self.widget_state.id, deadline)
        }
    }
);

impl EventCtx<'_, '_> {
    /// Submit a [`Notification`].
    ///
    /// The provided argument can be a [`Selector`] or a [`Command`]; this lets
    /// us work with the existing API for adding a payload to a [`Selector`].
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
        trace!("submit_notification");
        let note = note.into().into_notification(self.widget_state.id);
        self.notifications.push_back(note);
    }

    /// Submit a [`Notification`] without warning.
    ///
    /// In contrast to [`submit_notification`], calling this method will not result in an
    /// "unhandled notification" warning.
    ///
    /// [`submit_notification`]: crate::EventCtx::submit_notification
    //TODO: decide if we should use a known_target flag on submit_notification instead,
    // which would be a breaking change.
    pub fn submit_notification_without_warning(&mut self, note: impl Into<Command>) {
        trace!("submit_notification");
        let note = note
            .into()
            .into_notification(self.widget_state.id)
            .warn_if_unused(false);
        self.notifications.push_back(note);
    }

    /// Set the "active" state of the widget.
    ///
    /// See [`EventCtx::is_active`](struct.EventCtx.html#method.is_active).
    pub fn set_active(&mut self, active: bool) {
        trace!("set_active({})", active);
        self.widget_state.is_active = active;
        // TODO: plumb mouse grab through to platform (through druid-shell)
    }

    /// Create a new window.
    /// `T` must be the application's root `Data` type (the type provided to [`AppLauncher::launch`]).
    ///
    /// [`AppLauncher::launch`]: struct.AppLauncher.html#method.launch
    pub fn new_window<T: Any>(&mut self, desc: WindowDesc<T>) {
        trace!("new_window");
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
    pub fn show_context_menu<T: Any>(&mut self, menu: Menu<T>, location: Point) {
        trace!("show_context_menu");
        if self.state.root_app_data_type == TypeId::of::<T>() {
            let menu = ContextMenu { menu, location };
            self.submit_command(
                commands::SHOW_CONTEXT_MENU
                    .with(SingleUse::new(Box::new(menu)))
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
        trace!("set_handled");
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
        trace!("request_focus");
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
        trace!("set_focus target={:?}", target);
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
        trace!("focus_next");
        if self.has_focus() {
            self.widget_state.request_focus = Some(FocusChange::Next);
        } else {
            warn!(
                "focus_next can only be called by the currently \
                            focused widget or one of its ancestors."
            );
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
        trace!("focus_prev");
        if self.has_focus() {
            self.widget_state.request_focus = Some(FocusChange::Previous);
        } else {
            warn!(
                "focus_prev can only be called by the currently \
                            focused widget or one of its ancestors."
            );
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
        trace!("resign_focus");
        if self.has_focus() {
            self.widget_state.request_focus = Some(FocusChange::Resign);
        } else {
            warn!(
                "resign_focus can only be called by the currently focused widget \
                 or one of its ancestors. ({:?})",
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
        trace!("request_update");
        self.widget_state.request_update = true;
    }

    /// Scrolls the area into view.
    ///
    /// If the area is only partially visible or not visible at all because of [`Scroll`]s
    /// this widget is wrapped in, they will do the minimum amount of scrolling necessary to
    /// bring the area fully into view.
    ///
    /// If the widget is [`hidden`], this method has no effect.
    ///
    /// [`Scroll`]: crate::widget::Scroll
    /// [`hidden`]: crate::Event::should_propagate_to_hidden
    pub fn scroll_area_to_view(&mut self, area: Rect) {
        //TODO: only do something if this widget is not hidden
        self.submit_notification_without_warning(
            SCROLL_TO_VIEW.with(area + self.window_origin().to_vec2()),
        );
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

    /// Scrolls the area into view.
    ///
    /// If the area is only partially visible or not visible at all because of [`Scroll`]s
    /// this widget is wrapped in, they will do the minimum amount of scrolling necessary to
    /// bring the area fully into view.
    ///
    /// If the widget is [`hidden`], this method has no effect.
    ///
    /// [`Scroll`]: crate::widget::Scroll
    /// [`hidden`]: crate::Event::should_propagate_to_hidden
    pub fn scroll_area_to_view(&mut self, area: Rect) {
        //TODO: only do something if this widget is not hidden
        self.submit_command(Command::new(
            SCROLL_TO_VIEW,
            area + self.window_origin().to_vec2(),
            self.widget_id(),
        ));
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
        trace!("register_child id={:?}", child_id);
        self.widget_state.children.add(&child_id);
    }

    /// Register this widget to be eligile to accept focus automatically.
    ///
    /// This should only be called in response to a [`LifeCycle::BuildFocusChain`] event.
    ///
    /// See [`EventCtx::is_focused`] for more information about focus.
    ///
    /// [`LifeCycle::BuildFocusChain`]: enum.Lifecycle.html#variant.BuildFocusChain
    /// [`EventCtx::is_focused`]: struct.EventCtx.html#method.is_focused
    pub fn register_for_focus(&mut self) {
        trace!("register_for_focus");
        self.widget_state.focus_chain.push(self.widget_id());
    }

    /// Register this widget as accepting text input.
    pub fn register_text_input(&mut self, document: impl ImeHandlerRef + 'static) {
        let registration = TextFieldRegistration {
            document: Rc::new(document),
            widget_id: self.widget_id(),
        };
        self.state.text_registrations.push(registration);
    }

    /// Scrolls the area into view.
    ///
    /// If the area is only partially visible or not visible at all because of [`Scroll`]s
    /// this widget is wrapped in, they will do the minimum amount of scrolling necessary to
    /// bring the area fully into view.
    ///
    /// If the widget is [`hidden`], this method has no effect.
    ///
    /// [`Scroll`]: crate::widget::Scroll
    /// [`hidden`]: crate::Event::should_propagate_to_hidden
    pub fn scroll_area_to_view(&mut self, area: Rect) {
        //TODO: only do something if this widget is not hidden
        self.submit_command(
            SCROLL_TO_VIEW
                .with(area + self.window_origin().to_vec2())
                .to(self.widget_id()),
        );
    }
}

impl<'a, 'b> LayoutCtx<'a, 'b> {
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
        let insets = insets.into();
        trace!("set_paint_insets {:?}", insets);
        self.widget_state.paint_insets = insets.nonnegative();
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
        trace!("set_baseline_offset {}", baseline);
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
            error!("Failed to save RenderContext: '{}'", e);
            return;
        }

        f(self);

        if let Err(e) = self.render_ctx.restore() {
            error!("Failed to restore RenderContext: '{}'", e);
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
        timers: &'a mut HashMap<TimerToken, WidgetId>,
        text_registrations: &'a mut Vec<TextFieldRegistration>,
    ) -> Self {
        ContextState {
            command_queue,
            ext_handle,
            window,
            window_id,
            focus_widget,
            timers,
            text_registrations,
            text: window.text(),
            root_app_data_type: TypeId::of::<T>(),
        }
    }

    fn submit_command(&mut self, command: Command) {
        trace!("submit_command");
        self.command_queue
            .push_back(command.default_to(self.window_id.into()));
    }

    fn request_timer(&mut self, widget_id: WidgetId, deadline: Duration) -> TimerToken {
        trace!("request_timer deadline={:?}", deadline);
        let timer_token = self.window.request_timer(deadline);
        self.timers.insert(timer_token, widget_id);
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
