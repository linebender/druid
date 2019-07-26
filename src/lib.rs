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
use std::ops::DerefMut;
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
pub use druid_shell::window::{Cursor, MouseButton, MouseEvent};

pub use data::Data;
pub use event::{Event, WheelEvent};
pub use lens::{Lens, LensWrap};
pub use value::{Delta, KeyPath, PathEl, PathFragment, Value};

const BACKGROUND_COLOR: Color = Color::rgb24(0x27_28_22);

// We can probably get rid of the distinction between this and UiState.
pub struct UiMain<T: Data> {
    state: UiState<T>,
}

pub struct UiState<T: Data> {
    root: WidgetPod<T, Box<dyn Widget<T>>>,
    data: T,
    prev_paint_time: Option<Instant>,
    // Following fields might move to a separate struct so there's access
    // from contexts.
    handle: WindowHandle,
    size: Size,
}

pub struct WidgetPod<T: Data, W: Widget<T>> {
    state: BaseState,
    old_data: Option<T>,
    inner: W,
}

/// Convenience type for dynamic boxed widget.
pub type BoxedWidget<T> = WidgetPod<T, Box<dyn Widget<T>>>;

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

    /// This widget or a descendant has focus.
    has_focus: bool,

    /// This widget or a descendant has requested focus.
    request_focus: bool,
}

pub trait Widget<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env);

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size;

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut T,
        env: &Env,
    ) -> Option<Action>;

    // Consider a no-op default impl.
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

#[derive(Clone, Default)]
pub struct Env {
    value: Value,
    path: KeyPath,
}

pub struct PaintCtx<'a, 'b: 'a> {
    pub render_ctx: &'a mut Piet<'b>,
}

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

#[derive(Debug)]
pub struct Action {
    // This is just a placeholder for debugging purposes.
    text: String,
}

#[derive(Clone, Copy, Debug)]
pub struct BoxConstraints {
    min: Size,
    max: Size,
}

impl<T: Data, W: Widget<T>> WidgetPod<T, W> {
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

    pub fn get_layout_rect(&self) -> Rect {
        self.state.layout_rect
    }

    pub fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(paint_ctx, &self.state, data, &env);
    }

    /// Paint the widget, translating it by the origin of its layout rectangle.
    // Discussion: should this be `paint` and the other `paint_raw`?
    pub fn paint_with_offset(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Err(e) = paint_ctx.render_ctx.save() {
            eprintln!("error saving render context: {:?}", e);
            return;
        }
        paint_ctx
            .render_ctx
            .transform(Affine::translate(self.state.layout_rect.origin().to_vec2()));
        self.paint(paint_ctx, data, env);
        if let Err(e) = paint_ctx.render_ctx.restore() {
            eprintln!("error restoring render context: {:?}", e);
        }
    }

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
        ctx.base_state.is_hot |= child_ctx.base_state.is_hot;
        ctx.base_state.has_active |= child_ctx.base_state.has_active;
        ctx.base_state.request_focus |= child_ctx.base_state.request_focus;
        ctx.is_handled |= child_ctx.is_handled;
        action
    }

    /// Propagate a data update.
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

// Consider putting the `'static` bound on the main impl.
impl<T: Data, W: Widget<T> + 'static> WidgetPod<T, W> {
    pub fn boxed(self) -> BoxedWidget<T> {
        WidgetPod {
            state: self.state,
            old_data: self.old_data,
            inner: Box::new(self.inner),
        }
    }
}

// The following seems not to work because of the parametrization on T.
/*
// Convenience method for conversion to boxed widgets.
impl<T: Data, W: Widget<T> + 'static> From<W> for BoxedWidget<T> {
    fn from(w: W) -> BoxedWidget<T> {
        WidgetPod::new(w).boxed()
    }
}
*/

impl<T: Data> UiState<T> {
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
        let mut ctx = EventCtx {
            win_ctx,
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
        // TODO: issue anim_frame_event. Needs a win_ctx for this, which needs to be
        // plumbed.
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

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl BaseState {
    pub fn is_hot(&self) -> bool {
        self.is_hot
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn has_focus(&self) -> bool {
        self.has_focus
    }

    pub fn size(&self) -> Size {
        self.layout_rect.size()
    }
}

impl BoxConstraints {
    pub fn new(min: Size, max: Size) -> BoxConstraints {
        BoxConstraints { min, max }
    }

    pub fn tight(size: Size) -> BoxConstraints {
        BoxConstraints {
            min: size,
            max: size,
        }
    }

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

    /// Set the "active" state of the widget.
    ///
    /// The active state basically captures a mouse press inside the widget.
    /// Thus, a button should set this to true on mouse down and false on
    /// mouse up.
    ///
    /// While a widget is active, all mouse events are routed to it.
    pub fn set_active(&mut self, active: bool) {
        self.base_state.is_active = active;
        // TODO: plumb mouse grab through to platform (through druid-shell)
    }

    /// Query the "hot" state of the widget.
    ///
    /// A widget is hot when the mouse is hovering.
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

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        self.base_state.request_anim = true;
    }

    pub fn has_focus(&self) -> bool {
        self.base_state.has_focus
    }

    /// Request keyboard focus.
    ///
    /// Discussion question: is method needed in contexts other than event?
    pub fn request_focus(&mut self) {
        self.base_state.request_focus = true;
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
