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

//! Simple data-oriented GUI.

#![deny(intra_doc_link_resolution_failure, unsafe_code)]
#![allow(clippy::new_ret_no_self)]

use druid_shell as shell;
pub use druid_shell::{kurbo, piet};

mod app;
mod app_delegate;
mod command;
mod data;
mod env;
mod event;
pub mod lens;
mod localization;
mod menu;
mod mouse;
pub mod theme;
pub mod widget;
mod win_handler;
mod window;

use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::time::Instant;

use log::{error, warn};

use kurbo::{Affine, Point, Rect, Shape, Size, Vec2};
use piet::{Piet, RenderContext};

// these are the types from shell that we expose; others we only use internally.
pub use shell::{
    Application, Clipboard, ClipboardFormat, Cursor, FileDialogOptions, FileInfo, FileSpec,
    FormatId, HotKey, KeyCode, KeyEvent, KeyModifiers, MouseButton, RawMods, SysMods, Text,
    TimerToken, WinCtx, WindowHandle,
};

pub use app::{AppLauncher, WindowDesc};
pub use app_delegate::{AppDelegate, DelegateCtx};
pub use command::{sys as commands, Command, Selector};
pub use data::Data;
pub use env::{Env, Key, Value};
pub use event::{Event, LifeCycle, WheelEvent};
pub use lens::{Lens, LensExt, LensWrap};
pub use localization::LocalizedString;
pub use menu::{sys as platform_menus, ContextMenu, MenuDesc, MenuItem};
pub use mouse::MouseEvent;
pub use win_handler::DruidHandler;
pub use window::{Window, WindowId};

/// A container for one widget in the hierarchy.
///
/// Generally, container widgets don't contain other widgets directly,
/// but rather contain a `WidgetPod`, which has additional state needed
/// for layout and for the widget to participate in event flow.
///
/// This struct also contains the previous data for a widget, which is
/// essential for the [`update`] method, both to decide when the update
/// needs to propagate, and to provide the previous data so that a
/// widget can process a diff between the old value and the new.
///
/// [`update`]: trait.Widget.html#tymethod.update
pub struct WidgetPod<T: Data, W: Widget<T>> {
    state: BaseState,
    old_data: Option<T>,
    env: Option<Env>,
    inner: W,
}

/// Convenience type for dynamic boxed widget.
pub type BoxedWidget<T> = WidgetPod<T, Box<dyn Widget<T>>>;

/// Generic state for all widgets in the hierarchy.
///
/// This struct contains the widget's layout rect, flags
/// indicating when the widget is active or focused, and other
/// state necessary for the widget to participate in event
/// flow.
///
/// It is provided to [`paint`] calls as a non-mutable reference,
/// largely so a widget can know its size, also because active
/// and focus state can affect the widget's appearance. Other than
/// that, widgets will generally not interact with it directly,
/// but it is an important part of the [`WidgetPod`] struct.
///
/// [`paint`]: trait.Widget.html#tymethod.paint
/// [`WidgetPod`]: struct.WidgetPod.html
#[derive(Default)]
pub struct BaseState {
    layout_rect: Rect,

    // TODO: consider using bitflags for the booleans.

    // This should become an invalidation rect.
    needs_inval: bool,

    is_hot: bool,

    is_active: bool,

    /// Any descendant is active.
    has_active: bool,

    /// Any descendant has requested an animation frame.
    request_anim: bool,

    /// Any descendant has requested a timer.
    ///
    /// Note: we don't have any way of clearing this request, as it's
    /// likely not worth the complexity.
    request_timer: bool,

    /// This widget or a descendant has focus.
    has_focus: bool,

    /// This widget or a descendant has requested focus.
    request_focus: bool,
}

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
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env);
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

    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        self.deref_mut().paint(paint_ctx, base_state, data, env);
    }
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
    /// The currently visible region.
    pub(crate) region: Region,
}

/// A region of a widget, generally used to describe what needs to be drawn.
#[derive(Debug, Clone)]
pub struct Region(Rect);

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

impl<'a, 'b: 'a> PaintCtx<'a, 'b> {
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
        let PaintCtx {
            render_ctx,
            window_id,
            ..
        } = self;
        let mut child_ctx = PaintCtx {
            render_ctx,
            window_id: *window_id,
            region: region.into(),
        };
        f(&mut child_ctx)
    }
}

/// A context provided to layout handling methods of widgets.
///
/// As of now, the main service provided is access to a factory for
/// creating text layout objects, which are likely to be useful
/// during widget layout.
pub struct LayoutCtx<'a, 'b: 'a> {
    text_factory: &'a mut Text<'b>,
    window_id: WindowId,
}

/// A mutable context provided to event handling methods of widgets.
///
/// Widgets should call [`invalidate`] whenever an event causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`invalidate`]: #method.invalidate
pub struct EventCtx<'a, 'b> {
    // Note: there's a bunch of state that's just passed down, might
    // want to group that into a single struct.
    win_ctx: &'a mut dyn WinCtx<'b>,
    cursor: &'a mut Option<Cursor>,
    /// Commands submitted to be run after this event.
    command_queue: &'a mut VecDeque<(WindowId, Command)>,
    window_id: WindowId,
    // TODO: migrate most usage of `WindowHandle` to `WinCtx` instead.
    window: &'a WindowHandle,
    base_state: &'a mut BaseState,
    had_active: bool,
    is_handled: bool,
    is_root: bool,
}

/// A mutable context provided to data update methods of widgets.
///
/// Widgets should call [`invalidate`] whenever a data change causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`invalidate`]: #method.invalidate
pub struct UpdateCtx<'a, 'b: 'a> {
    text_factory: &'a mut Text<'b>,
    window: &'a WindowHandle,
    // Discussion: we probably want to propagate more fine-grained
    // invalidations, which would mean a structure very much like
    // `EventCtx` (and possibly using the same structure). But for
    // now keep it super-simple.
    needs_inval: bool,
    window_id: WindowId,
}

/// Constraints for layout.
///
/// The layout strategy for druid is strongly inspired by Flutter,
/// and this struct is similar to the [Flutter BoxConstraints] class.
///
/// At the moment, it represents simply a minimum and maximum size.
/// A widget's [`layout`] method should choose an appropriate size that
/// meets these constraints.
///
/// Further, a container widget should compute appropriate constraints
/// for each of its child widgets, and pass those down when recursing.
///
/// [`layout`]: trait.Widget.html#tymethod.layout
/// [Flutter BoxConstraints]: https://api.flutter.dev/flutter/rendering/BoxConstraints-class.html
#[derive(Clone, Copy, Debug)]
pub struct BoxConstraints {
    min: Size,
    max: Size,
}

impl<T: Data, W: Widget<T>> WidgetPod<T, W> {
    /// Create a new widget pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `WidgetPod`
    /// so it can participate in layout and event flow. The process of
    /// adding a child widget to a container should call this method.
    pub fn new(inner: W) -> WidgetPod<T, W> {
        WidgetPod {
            state: Default::default(),
            old_data: None,
            env: None,
            inner,
        }
    }

    /// Return a reference to the inner widget.
    pub fn widget(&self) -> &W {
        &self.inner
    }

    /// Return a mutable reference to the inner widget.
    pub fn widget_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Set layout rectangle.
    ///
    /// Intended to be called on child widget in container's `layout`
    /// implementation.
    pub fn set_layout_rect(&mut self, layout_rect: Rect) {
        self.state.layout_rect = layout_rect;
    }

    /// Get the layout rectangle.
    ///
    /// This will be same value as set by `set_layout_rect`.
    pub fn get_layout_rect(&self) -> Rect {
        self.state.layout_rect
    }

    /// Paint a child widget.
    ///
    /// Generally called by container widgets as part of their [`paint`]
    /// method.
    ///
    /// Note that this method does not apply the offset of the layout rect.
    /// If that is desired, use [`paint_with_offset`] instead.
    ///
    /// [`layout`]: trait.Widget.html#method.layout
    /// [`paint`]: trait.Widget.html#method.paint
    /// [`paint_with_offset`]: #method.paint_with_offset
    pub fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(paint_ctx, &self.state, data, &env);
    }

    /// Paint the widget, translating it by the origin of its layout rectangle.
    ///
    /// This will recursively paint widgets, stopping if a widget's layout
    /// rect is outside of the currently visible region.
    // Discussion: should this be `paint` and the other `paint_raw`?
    pub fn paint_with_offset(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.paint_with_offset_impl(paint_ctx, data, env, false)
    }

    /// Paint the widget, even if its layout rect is outside of the currently
    /// visible region.
    pub fn paint_with_offset_always(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.paint_with_offset_impl(paint_ctx, data, env, true)
    }

    /// Shared implementation that can skip drawing non-visible content.
    fn paint_with_offset_impl(
        &mut self,
        paint_ctx: &mut PaintCtx,
        data: &T,
        env: &Env,
        paint_if_not_visible: bool,
    ) {
        if !paint_if_not_visible && !paint_ctx.region().intersects(self.state.layout_rect) {
            return;
        }

        if let Err(e) = paint_ctx.save() {
            error!("saving render context failed: {:?}", e);
            return;
        }

        let layout_origin = self.state.layout_rect.origin().to_vec2();
        paint_ctx.transform(Affine::translate(layout_origin));

        let visible = paint_ctx.region().to_rect() - layout_origin;

        paint_ctx.with_child_ctx(visible, |ctx| {
            self.inner.paint(ctx, &self.state, data, &env)
        });

        if let Err(e) = paint_ctx.restore() {
            error!("restoring render context failed: {:?}", e);
        }
    }

    /// Compute layout of a widget.
    ///
    /// Generally called by container widgets as part of their [`layout`]
    /// method.
    ///
    /// [`layout`]: trait.Widget.html#method.layout
    pub fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        self.inner.layout(layout_ctx, bc, data, &env)
    }

    /// Propagate an event.
    ///
    /// Generally the [`event`] method of a container widget will call this
    /// method on all its children. Here is where a great deal of the event
    /// flow logic resides, particularly whether to continue propagating
    /// the event.
    ///
    /// [`event`]: trait.Widget.html#method.event
    pub fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        // TODO: factor as much logic as possible into monomorphic functions.
        if ctx.is_handled || !event.recurse() {
            // This function is called by containers to propagate an event from
            // containers to children. Non-recurse events will be invoked directly
            // from other points in the library.
            return;
        }
        let had_active = self.state.has_active;
        let mut child_ctx = EventCtx {
            win_ctx: ctx.win_ctx,
            cursor: ctx.cursor,
            command_queue: ctx.command_queue,
            window: &ctx.window,
            window_id: ctx.window_id,
            base_state: &mut self.state,
            had_active,
            is_handled: false,
            is_root: false,
        };
        let rect = child_ctx.base_state.layout_rect;
        // Note: could also represent this as `Option<Event>`.
        let mut recurse = true;
        let mut hot_changed = None;
        let child_event = match event {
            Event::LifeCycle(event) => Event::LifeCycle(*event),
            Event::Size(size) => {
                recurse = ctx.is_root;
                Event::Size(*size)
            }
            Event::MouseDown(mouse_event) => {
                let had_hot = child_ctx.base_state.is_hot;
                let now_hot = rect.winding(mouse_event.pos) != 0;
                if (!had_hot) && now_hot {
                    child_ctx.base_state.is_hot = true;
                    hot_changed = Some(true);
                }
                recurse = had_active || !ctx.had_active && now_hot;
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos -= rect.origin().to_vec2();
                Event::MouseDown(mouse_event)
            }
            Event::MouseUp(mouse_event) => {
                recurse = had_active || !ctx.had_active && rect.winding(mouse_event.pos) != 0;
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos -= rect.origin().to_vec2();
                Event::MouseUp(mouse_event)
            }
            Event::MouseMoved(mouse_event) => {
                let had_hot = child_ctx.base_state.is_hot;
                child_ctx.base_state.is_hot = rect.winding(mouse_event.pos) != 0;
                if had_hot != child_ctx.base_state.is_hot {
                    hot_changed = Some(child_ctx.base_state.is_hot);
                }
                recurse = had_active || had_hot || child_ctx.base_state.is_hot;
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos -= rect.origin().to_vec2();
                Event::MouseMoved(mouse_event)
            }
            Event::KeyDown(e) => {
                recurse = child_ctx.base_state.has_focus;
                Event::KeyDown(*e)
            }
            Event::KeyUp(e) => {
                recurse = child_ctx.base_state.has_focus;
                Event::KeyUp(*e)
            }
            Event::Paste(e) => {
                recurse = child_ctx.base_state.has_focus;
                Event::Paste(e.clone())
            }
            Event::Wheel(wheel_event) => {
                recurse = had_active || child_ctx.base_state.is_hot;
                Event::Wheel(wheel_event.clone())
            }
            Event::HotChanged(is_hot) => Event::HotChanged(*is_hot),
            Event::FocusChanged(_is_focused) => {
                let had_focus = child_ctx.base_state.has_focus;
                let focus = child_ctx.base_state.request_focus;
                child_ctx.base_state.request_focus = false;
                child_ctx.base_state.has_focus = focus;
                recurse = focus || had_focus;
                Event::FocusChanged(focus)
            }
            Event::AnimFrame(interval) => {
                recurse = child_ctx.base_state.request_anim;
                child_ctx.base_state.request_anim = false;
                Event::AnimFrame(*interval)
            }
            Event::Timer(id) => {
                recurse = child_ctx.base_state.request_timer;
                Event::Timer(*id)
            }
            Event::Command(cmd) => Event::Command(cmd.clone()),
        };
        child_ctx.base_state.needs_inval = false;
        if let Some(is_hot) = hot_changed {
            let hot_changed_event = Event::HotChanged(is_hot);
            self.inner
                .event(&mut child_ctx, &hot_changed_event, data, &env);
        }
        if recurse {
            child_ctx.base_state.has_active = false;
            self.inner.event(&mut child_ctx, &child_event, data, &env);
            child_ctx.base_state.has_active |= child_ctx.base_state.is_active;
        };
        ctx.base_state.needs_inval |= child_ctx.base_state.needs_inval;
        ctx.base_state.request_anim |= child_ctx.base_state.request_anim;
        ctx.base_state.request_timer |= child_ctx.base_state.request_timer;
        ctx.base_state.is_hot |= child_ctx.base_state.is_hot;
        ctx.base_state.has_active |= child_ctx.base_state.has_active;
        ctx.base_state.request_focus |= child_ctx.base_state.request_focus;
        ctx.is_handled |= child_ctx.is_handled;
    }

    /// Propagate a data update.
    ///
    /// Generally called by container widgets as part of their [`update`]
    /// method.
    ///
    /// [`update`]: trait.Widget.html#method.update
    pub fn update(&mut self, ctx: &mut UpdateCtx, data: &T, env: &Env) {
        let data_same = if let Some(ref old_data) = self.old_data {
            old_data.same(data)
        } else {
            false
        };
        let env_same = if let Some(ref old_env) = self.env {
            old_env.same(env)
        } else {
            false
        };

        if data_same && env_same {
            return;
        }
        self.inner.update(ctx, self.old_data.as_ref(), data, env);
        self.old_data = Some(data.clone());
        self.env = Some(env.clone());
    }
}

impl<T: Data, W: Widget<T> + 'static> WidgetPod<T, W> {
    /// Box the contained widget.
    ///
    /// Convert a `WidgetPod` containing a widget of a specific concrete type
    /// into a dynamically boxed widget.
    pub fn boxed(self) -> BoxedWidget<T> {
        WidgetPod {
            state: self.state,
            old_data: self.old_data,
            env: self.env,
            inner: Box::new(self.inner),
        }
    }
}

impl BaseState {
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
        self.is_hot
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
        self.is_active
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
        self.has_focus
    }

    /// The layout size.
    ///
    /// This is the layout size as ultimately determined by the parent
    /// container. Generally it will be the same as the size returned by
    /// the child widget's [`layout`] method.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    pub fn size(&self) -> Size {
        self.layout_rect.size()
    }
}

impl BoxConstraints {
    /// Create a new box constraints object.
    ///
    /// Create constraints based on minimum and maximum size.
    pub fn new(min: Size, max: Size) -> BoxConstraints {
        BoxConstraints { min, max }
    }

    /// Create a "tight" box constraints object.
    ///
    /// A "tight" constraint can only be satisfied by a single size.
    pub fn tight(size: Size) -> BoxConstraints {
        BoxConstraints {
            min: size,
            max: size,
        }
    }

    /// Create a "loose" version of the constraints.
    ///
    /// Make a version with zero minimum size, but the same maximum size.
    pub fn loosen(&self) -> BoxConstraints {
        BoxConstraints {
            min: Size::ZERO,
            max: self.max,
        }
    }

    /// Clamp a given size so that fits within the constraints.
    pub fn constrain(&self, size: impl Into<Size>) -> Size {
        size.into().clamp(self.min, self.max)
    }

    /// Returns the max size of these constraints.
    pub fn max(&self) -> Size {
        self.max
    }

    /// Returns the min size of these constraints.
    pub fn min(&self) -> Size {
        self.min
    }

    /// Whether there is an upper bound on the width.
    pub fn is_width_bounded(&self) -> bool {
        self.max.width.is_finite()
    }

    /// Whether there is an upper bound on the height.
    pub fn is_height_bounded(&self) -> bool {
        self.max.height.is_finite()
    }

    /// Check to see if these constraints are legit.
    ///
    /// Logs a warning if BoxConstraints are invalid.
    pub fn debug_check(&self, name: &str) {
        if !(0.0 <= self.min.width
            && self.min.width <= self.max.width
            && 0.0 <= self.min.height
            && self.min.height <= self.max.height)
        {
            warn!("Bad BoxConstraints passed to {}:", name);
            warn!("{:?}", self);
        }
    }

    /// Shrink min and max constraints by size
    pub fn shrink(&self, diff: impl Into<Size>) -> BoxConstraints {
        let diff = diff.into();
        let min = Size::new(
            (self.min().width - diff.width).max(0.),
            (self.min().height - diff.height).max(0.),
        );
        let max = Size::new(
            (self.max().width - diff.width).max(0.),
            (self.max().height - diff.height).max(0.),
        );

        BoxConstraints::new(min, max)
    }
}

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
    /// See [`BaseState::is_active`](struct.BaseState.html#method.is_active).
    pub fn set_active(&mut self, active: bool) {
        self.base_state.is_active = active;
        // TODO: plumb mouse grab through to platform (through druid-shell)
    }

    /// Query the "hot" state of the widget.
    ///
    /// See [`BaseState::is_hot`](struct.BaseState.html#method.is_hot).
    pub fn is_hot(&self) -> bool {
        self.base_state.is_hot
    }

    /// Query the "active" state of the widget.
    ///
    /// This is the same state set by [`set_active`](#method.set_active) and
    /// is provided as a convenience.
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

    /// Query the focus state of the widget.
    ///
    /// See [`BaseState::has_focus`](struct.BaseState.html#method.has_focus).
    pub fn has_focus(&self) -> bool {
        self.base_state.has_focus
    }

    /// Request keyboard focus.
    ///
    /// Discussion question: is method needed in contexts other than event?
    pub fn request_focus(&mut self) {
        self.base_state.request_focus = true;
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        self.base_state.request_anim = true;
    }

    /// Request a timer event.
    ///
    /// The return value is a token, which can be used to associate the
    /// request with the event.
    pub fn request_timer(&mut self, deadline: Instant) -> TimerToken {
        self.base_state.request_timer = true;
        self.win_ctx.request_timer(deadline)
    }

    /// Returns the layout size of the current widget.
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
        window_id: impl Into<Option<WindowId>>,
    ) {
        let window_id = window_id.into().unwrap_or(self.window_id);
        self.command_queue.push_back((window_id, command.into()))
    }

    /// Get the window id.
    pub fn window_id(&self) -> WindowId {
        self.window_id
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

impl<'a, 'b> UpdateCtx<'a, 'b> {
    /// Invalidate.
    ///
    /// See [`EventCtx::invalidate`](struct.EventCtx.html#method.invalidate) for
    /// more discussion.
    pub fn invalidate(&mut self) {
        self.needs_inval = true;
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
    pub fn window(&self) -> &WindowHandle {
        &self.window
    }

    /// Get the window id.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }
}
