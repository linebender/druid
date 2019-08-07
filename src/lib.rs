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

pub use druid_shell::{self as shell, kurbo, piet};

pub mod widget;

mod data;
mod event;
mod lens;
mod value;

use std::any::Any;
use std::ops::{Deref, DerefMut};

use std::time::Instant;

use kurbo::{Affine, Point, Rect, Shape, Size, Vec2};
use piet::{Color, Piet, RenderContext};

// TODO: remove these unused annotations when we wire these up; they're
// placeholders for functionality not yet implemented.
#[allow(unused)]
use druid_shell::application::Application;
pub use druid_shell::dialog::{FileDialogOptions, FileDialogType};
pub use druid_shell::keyboard::{KeyCode, KeyEvent, KeyModifiers};
#[allow(unused)]
use druid_shell::platform::IdleHandle;
use druid_shell::window::{self, Text, WinCtx, WinHandler, WindowHandle};
pub use druid_shell::window::{Cursor, MouseButton, MouseEvent, TimerToken};

pub use data::Data;
pub use event::{Event, WheelEvent};
pub use lens::{Lens, LensWrap};
pub use value::{Delta, KeyPath, PathEl, PathFragment, Value};

const BACKGROUND_COLOR: Color = Color::rgb24(0x27_28_22);

/// A struct representing the top-level root of the UI.
///
/// At the moment, there is no meaningful distinction between this struct
/// and [`UiState`].
///
/// Discussion: when we start supporting multiple windows, we'll need
/// to make finer distinctions between state for the entire application,
/// and state for a single window. But for now it's the same.
pub struct UiMain<T: Data> {
    state: UiState<T>,
}

/// The state of the top-level UI.
///
/// This struct holds the root widget of the UI, and is also responsible
/// for coordinating interactions with the platform window.
pub struct UiState<T: Data> {
    root: WidgetPod<T, Box<dyn Widget<T>>>,
    data: T,
    prev_paint_time: Option<Instant>,
    // Following fields might move to a separate struct so there's access
    // from contexts.
    handle: WindowHandle,
    size: Size,
}

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

    /// Handle an event.
    ///
    /// A number of different events (in the [`Event`] enum) are handled in this
    /// method call. A widget can handle these events in a number of ways:
    /// requesting things from the [`EventCtx`], mutating the data, or returning
    /// an [`Action`].
    ///
    /// [`Event`]: struct.Event.html
    /// [`EventCtx`]: struct.EventCtx.html
    /// [`Action`]: struct.Action.html
    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut T,
        env: &Env,
    ) -> Option<Action>;

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
}

// TODO: explore getting rid of this (ie be consistent about using
// `dyn Widget` only).
impl<T> Widget<T> for Box<dyn Widget<T>> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        self.deref_mut().paint(paint_ctx, base_state, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.deref_mut().layout(ctx, bc, data, env)
    }

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut T,
        env: &Env,
    ) -> Option<Action> {
        self.deref_mut().event(event, ctx, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        self.deref_mut().update(ctx, old_data, data, env);
    }
}

/// An environment passed down through all widget traversals.
///
/// All widget methods have access to an environment, and it is passed
/// downwards during traversals.
///
/// At present, there is no real functionality here, but work is in
/// progress to make theme data (colors, dimensions, etc) available
/// through the environment, as well as pass custom data down to all
/// descendants. An important example of the latter is setting a value
/// for enabled/disabled status so that an entire subtree can be
/// disabled ("grayed out") with one setting.
#[derive(Clone, Default)]
pub struct Env {
    value: Value,
    path: KeyPath,
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

/// A context provided to layout handling methods of widgets.
///
/// As of now, the main service provided is access to a factory for
/// creating text layout objects, which are likely to be useful
/// during widget layout.
pub struct LayoutCtx<'a, 'b: 'a> {
    text: &'a mut Text<'b>,
}

/// A mutable context provided to event handling methods of widgets.
///
/// Widgets should call [`invalidate`] whenever an event causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`invalidate`]: #method.invalidate
pub struct EventCtx<'a, 'b> {
    win_ctx: &'a mut dyn WinCtx<'b>,
    cursor: &'a mut Option<Cursor>,
    // TODO: migrate most usage of `WindowHandle` to `WinCtx` instead.
    window: &'a WindowHandle,
    base_state: &'a mut BaseState,
    had_active: bool,
    is_handled: bool,
}

/// A mutable context provided to data update methods of widgets.
///
/// Widgets should call [`invalidate`] whenever a data change causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`invalidate`]: #method.invalidate
pub struct UpdateCtx<'a, 'b> {
    win_ctx: &'a mut dyn WinCtx<'b>,
    window: &'a WindowHandle,
    // Discussion: we probably want to propagate more fine-grained
    // invalidations, which would mean a structure very much like
    // `EventCtx` (and possibly using the same structure). But for
    // now keep it super-simple.
    needs_inval: bool,
}

/// An action produced by a widget.
///
/// Widgets have several ways of producing an effect in response to
/// events. Two of these are mutating their data, and requesting
/// actions from [`EventCtx`]. When neither of those is suitable,
/// and the action is generic (for example, a button press), then
/// the event handler for a widget can return an `Action`, and it
/// is passed up the calling hierarchy.
///
/// The details of the contents of this struct are still subject to
/// change. It's also possible that the concept will go away; a
/// reasonable replacement is to provide buttons with a closure that
/// can perform the action more directly.
///
/// [`EventCtx`]: struct.EventCtx.html
#[derive(Debug)]
pub struct Action {
    // This is just a placeholder for debugging purposes.
    text: String,
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
            inner,
        }
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
    /// If that is desired, use [`paint_with_offset`](#method.paint_with_offset)
    /// instead.
    ///
    /// [`layout`]: trait.Widget.html#method.layout
    pub fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(paint_ctx, &self.state, data, &env);
    }

    /// Paint the widget, translating it by the origin of its layout rectangle.
    // Discussion: should this be `paint` and the other `paint_raw`?
    pub fn paint_with_offset(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Err(e) = paint_ctx.save() {
            eprintln!("error saving render context: {:?}", e);
            return;
        }
        paint_ctx.transform(Affine::translate(self.state.layout_rect.origin().to_vec2()));
        self.paint(paint_ctx, data, env);
        if let Err(e) = paint_ctx.restore() {
            eprintln!("error restoring render context: {:?}", e);
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
    pub fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut T,
        env: &Env,
    ) -> Option<Action> {
        // TODO: factor as much logic as possible into monomorphic functions.
        if ctx.is_handled || !event.recurse() {
            // This function is called by containers to propagate an event from
            // containers to children. Non-recurse events will be invoked directly
            // from other points in the library.
            return None;
        }
        let had_active = self.state.has_active;
        let mut child_ctx = EventCtx {
            win_ctx: ctx.win_ctx,
            cursor: ctx.cursor,
            window: &ctx.window,
            base_state: &mut self.state,
            had_active,
            is_handled: false,
        };
        let rect = child_ctx.base_state.layout_rect;
        // Note: could also represent this as `Option<Event>`.
        let mut recurse = true;
        let mut hot_changed = None;
        let child_event = match event {
            Event::MouseDown(mouse_event) => {
                recurse = had_active || !ctx.had_active && rect.winding(mouse_event.pos) != 0;
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
        };
        child_ctx.base_state.needs_inval = false;
        if let Some(is_hot) = hot_changed {
            let hot_changed_event = Event::HotChanged(is_hot);
            // Hot changed events are not expected to return an action.
            let _action = self
                .inner
                .event(&hot_changed_event, &mut child_ctx, data, &env);
        }
        let action = if recurse {
            child_ctx.base_state.has_active = false;
            let action = self.inner.event(&child_event, &mut child_ctx, data, &env);
            child_ctx.base_state.has_active |= child_ctx.base_state.is_active;
            action
        } else {
            None
        };
        ctx.base_state.needs_inval |= child_ctx.base_state.needs_inval;
        ctx.base_state.request_anim |= child_ctx.base_state.request_anim;
        ctx.base_state.request_timer |= child_ctx.base_state.request_timer;
        ctx.base_state.is_hot |= child_ctx.base_state.is_hot;
        ctx.base_state.has_active |= child_ctx.base_state.has_active;
        ctx.base_state.request_focus |= child_ctx.base_state.request_focus;
        ctx.is_handled |= child_ctx.is_handled;
        action
    }

    /// Propagate a data update.
    ///
    /// Generally called by container widgets as part of their [`update`]
    /// method.
    ///
    /// [`update`]: trait.Widget.html#method.update
    pub fn update(&mut self, ctx: &mut UpdateCtx, data: &T, env: &Env) {
        if let Some(old_data) = &self.old_data {
            if old_data.same(data) {
                return;
            }
        }
        self.inner.update(ctx, self.old_data.as_ref(), data, env);
        self.old_data = Some(data.clone());
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
            inner: Box::new(self.inner),
        }
    }
}

impl<T: Data> UiState<T> {
    /// Construct a new UI state.
    ///
    /// This constructor takes a root widget and an initial value for the
    /// data.
    pub fn new(root: impl Widget<T> + 'static, data: T) -> UiState<T> {
        UiState {
            root: WidgetPod::new(root).boxed(),
            data,
            prev_paint_time: None,
            handle: Default::default(),
            size: Default::default(),
        }
    }

    /// Set the root widget as active.
    ///
    /// Warning: this is set as deprecated because it's not really meaningful.
    /// It's likely that the intent was to set a default focus, but focus is
    /// not yet implemented and there probably needs to be some other way to
    /// identify the widget which should receive focus on startup.
    #[deprecated]
    pub fn set_active(&mut self, active: bool) {
        self.root.state.is_active = active;
    }

    fn root_env(&self) -> Env {
        Default::default()
    }

    /// Send an event to the widget hierarchy.
    ///
    /// Returns `true` if the event produced an action.
    ///
    /// This is principally because in certain cases (such as keydown on Windows)
    /// the OS needs to know if an event was handled.
    fn do_event(&mut self, event: Event, win_ctx: &mut dyn WinCtx) -> bool {
        let (is_handled, dirty) = self.do_event_inner(event, win_ctx);
        if dirty {
            win_ctx.invalidate();
        }
        is_handled
    }

    /// Send an event to the widget hierarchy.
    ///
    /// Returns two flags. The first is true if the event was handled. The
    /// second is true if an animation frame or invalidation is requested.
    fn do_event_inner(&mut self, event: Event, win_ctx: &mut dyn WinCtx) -> (bool, bool) {
        // should there be a root base state persisting in the ui state instead?
        let mut base_state = Default::default();
        let mut cursor = match event {
            Event::MouseMoved(..) => Some(Cursor::Arrow),
            _ => None,
        };
        let mut ctx = EventCtx {
            win_ctx,
            cursor: &mut cursor,
            window: &self.handle,
            base_state: &mut base_state,
            had_active: self.root.state.has_active,
            is_handled: false,
        };
        let env = self.root_env();
        let _action = self.root.event(&event, &mut ctx, &mut self.data, &env);

        if ctx.base_state.request_focus {
            let focus_event = Event::FocusChanged(true);
            // Focus changed events are not expected to return an action.
            let _ = self
                .root
                .event(&focus_event, &mut ctx, &mut self.data, &env);
        }
        let needs_inval = ctx.base_state.needs_inval;
        let request_anim = ctx.base_state.request_anim;
        let is_handled = ctx.is_handled();
        if let Some(cursor) = cursor {
            win_ctx.set_cursor(&cursor);
        }

        let mut update_ctx = UpdateCtx {
            win_ctx,
            window: &self.handle,
            needs_inval: false,
        };
        // Note: we probably want to aggregate updates so there's only one after
        // a burst of events.
        self.root.update(&mut update_ctx, &self.data, &env);
        // TODO: process actions
        let dirty = request_anim || needs_inval || update_ctx.needs_inval;
        (is_handled, dirty)
    }

    fn paint(&mut self, piet: &mut Piet, ctx: &mut dyn WinCtx) -> bool {
        // TODO: this calculation uses wall-clock time of the paint call, which
        // potentially has jitter.
        //
        // See https://github.com/xi-editor/druid/issues/85 for discussion.
        let this_paint_time = Instant::now();
        let interval = if let Some(last) = self.prev_paint_time {
            let duration = this_paint_time.duration_since(last);
            1_000_000_000 * duration.as_secs() + (duration.subsec_nanos() as u64)
        } else {
            0
        };
        let anim_frame_event = Event::AnimFrame(interval);
        let (_, request_anim) = self.do_event_inner(anim_frame_event, ctx);
        self.prev_paint_time = Some(this_paint_time);
        let bc = BoxConstraints::tight(self.size);
        let env = self.root_env();
        let text = piet.text();
        let mut layout_ctx = LayoutCtx { text };
        let size = self.root.layout(&mut layout_ctx, &bc, &self.data, &env);
        self.root.state.layout_rect = Rect::from_origin_size(Point::ORIGIN, size);
        piet.clear(BACKGROUND_COLOR);
        let mut paint_ctx = PaintCtx { render_ctx: piet };
        self.root.paint(&mut paint_ctx, &self.data, &env);
        if !request_anim {
            self.prev_paint_time = None;
        }
        request_anim
    }
}

impl<T: Data> UiMain<T> {
    /// Construct a new UI state.
    pub fn new(state: UiState<T>) -> UiMain<T> {
        UiMain { state }
    }
}

impl<T: Data + 'static> WinHandler for UiMain<T> {
    fn connect(&mut self, handle: &WindowHandle) {
        self.state.handle = handle.clone();
    }

    fn paint(&mut self, piet: &mut Piet, ctx: &mut dyn WinCtx) -> bool {
        self.state.paint(piet, ctx)
    }

    fn size(&mut self, width: u32, height: u32, _ctx: &mut dyn WinCtx) {
        let dpi = self.state.handle.get_dpi() as f64;
        let scale = 96.0 / dpi;
        self.state.size = Size::new(width as f64 * scale, height as f64 * scale);
    }

    fn mouse_down(&mut self, event: &window::MouseEvent, ctx: &mut dyn WinCtx) {
        // TODO: double-click detection
        let event = Event::MouseDown(event.clone());
        self.state.do_event(event, ctx);
    }

    fn mouse_up(&mut self, event: &MouseEvent, ctx: &mut dyn WinCtx) {
        let event = Event::MouseUp(event.clone());
        self.state.do_event(event, ctx);
    }

    fn mouse_move(&mut self, event: &MouseEvent, ctx: &mut dyn WinCtx) {
        let event = Event::MouseMoved(event.clone());
        self.state.do_event(event, ctx);
    }

    fn key_down(&mut self, event: KeyEvent, ctx: &mut dyn WinCtx) -> bool {
        self.state.do_event(Event::KeyDown(event), ctx)
    }

    fn key_up(&mut self, event: KeyEvent, ctx: &mut dyn WinCtx) {
        self.state.do_event(Event::KeyUp(event), ctx);
    }

    fn wheel(&mut self, delta: Vec2, mods: KeyModifiers, ctx: &mut dyn WinCtx) {
        let event = Event::Wheel(WheelEvent { delta, mods });
        self.state.do_event(event, ctx);
    }

    fn timer(&mut self, token: TimerToken, ctx: &mut dyn WinCtx) {
        self.state.do_event(Event::Timer(token), ctx);
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
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
}

impl Env {
    pub fn join(&self, fragment: impl PathFragment) -> Env {
        let mut path = self.path.clone();
        fragment.push_to_path(&mut path);
        // TODO: better diagnostics on error
        let value = self.value.access(fragment).expect("invalid path").clone();
        Env { value, path }
    }

    pub fn get_data(&self) -> &Value {
        &self.value
    }

    pub fn get_path(&self) -> &KeyPath {
        &self.path
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
    /// See [`BaseState::is_active`](struct.BaseState.html#method.is_hot).
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
}

impl<'a, 'b> LayoutCtx<'a, 'b> {
    /// Get an object which can create text layouts.
    pub fn text(&mut self) -> &mut Text<'b> {
        &mut self.text
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
        self.win_ctx.text_factory()
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
}

impl Action {
    /// Make an action from a string.
    ///
    /// Note: this is something of a placeholder and will change.
    pub fn from_str(s: impl Into<String>) -> Action {
        Action { text: s.into() }
    }

    /// Provides access to the action's string representation.
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }

    /// Merge two optional actions.
    ///
    /// Note: right now we're not dealing with the case where the event propagation
    /// results in more than one action. We need to rethink this.
    pub fn merge(this: Option<Action>, other: Option<Action>) -> Option<Action> {
        if this.is_some() {
            assert!(other.is_none(), "can't merge two actions");
            this
        } else {
            other
        }
    }
}
