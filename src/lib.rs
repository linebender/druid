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
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::time::Instant;

use kurbo::{Affine, Point, Rect, Shape, Size, Vec2};
use piet::{Color, Piet, RenderContext};

use druid_shell::application::Application;
pub use druid_shell::dialog::{FileDialogOptions, FileDialogType};
pub use druid_shell::keyboard::{KeyCode, KeyEvent, KeyModifiers};
use druid_shell::platform::IdleHandle;
use druid_shell::window::{self, WinHandler, WindowHandle};
pub use druid_shell::window::{Cursor, MouseButton, MouseEvent};

pub use data::Data;
pub use event::{Event, WheelEvent};
pub use lens::{Lens, LensWrap};
pub use value::{Delta, KeyPath, PathEl, PathFragment, Value};

const BACKGROUND_COLOR: Color = Color::rgb24(0x27_28_22);

pub struct UiMain<T: Data> {
    state: RefCell<UiState<T>>,
}

pub struct UiState<T: Data> {
    root: WidgetPod<T, Box<dyn Widget<T>>>,
    data: T,
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

    // This should become an invalidation rect.
    needs_inval: bool,

    // TODO: consider using bitflags.
    is_hot: bool,

    is_active: bool,

    /// Any descendant is active.
    has_active: bool,
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

pub struct LayoutCtx {}

pub struct EventCtx<'a> {
    window: &'a WindowHandle,
    base_state: &'a mut BaseState,
    had_active: bool,
    is_handled: bool,
}

pub struct UpdateCtx<'a> {
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
        if ctx.is_handled || !event.recurse() {
            // This function is called by containers to propagate an event from
            // containers to children. Non-recurse events will be invoked directly
            // from other points in the library.
            return None;
        }
        let had_active = self.state.has_active;
        let mut child_ctx = EventCtx {
            window: &ctx.window,
            base_state: &mut self.state,
            had_active,
            is_handled: false,
        };
        let rect = child_ctx.base_state.layout_rect;
        // Note: could also represent this as `Option<Event>`.
        let mut recurse = true;
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
                recurse = had_active || had_hot || child_ctx.base_state.is_hot;
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos -= rect.origin().to_vec2();
                Event::MouseMoved(mouse_event)
            }
            Event::KeyDown(_) | Event::KeyUp(_) if !had_active => return None,
            Event::KeyDown(e) => Event::KeyDown(*e),
            Event::KeyUp(e) => Event::KeyUp(*e),
            Event::Wheel(wheel_event) => {
                recurse = had_active || child_ctx.base_state.is_hot;
                Event::Wheel(wheel_event.clone())
            }
            Event::HotChanged(is_hot) => Event::HotChanged(*is_hot),
        };
        child_ctx.base_state.needs_inval = false;
        let action = if recurse {
            child_ctx.base_state.has_active = false;
            let action = self.inner.event(&child_event, &mut child_ctx, data, &env);
            child_ctx.base_state.has_active |= child_ctx.base_state.is_active;
            action
        } else {
            None
        };
        ctx.base_state.needs_inval |= child_ctx.base_state.needs_inval;
        ctx.base_state.is_hot |= child_ctx.base_state.is_hot;
        ctx.base_state.has_active |= child_ctx.base_state.has_active;
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
            handle: Default::default(),
            size: Default::default(),
        }
    }

    /// Set the root widget as active, that is, receiving keyboard events.
    pub fn set_active(&mut self, active: bool) {
        self.root.state.is_active = active;
    }

    fn root_env(&self) -> Env {
        Default::default()
    }

    /// Returns `true` if the event produced an action.
    ///
    /// This is principally because in certain cases (such as keydown on windows)
    /// the OS needs to know if an event was handled.
    fn do_event(&mut self, event: Event) -> bool {
        // should there be a root base state persisting in the ui state instead?
        let mut base_state = Default::default();
        let mut ctx = EventCtx {
            window: &self.handle,
            base_state: &mut base_state,
            had_active: self.root.state.has_active,
            is_handled: false,
        };
        let env = self.root_env();
        let action = self.root.event(&event, &mut ctx, &mut self.data, &env);
        let mut update_ctx = UpdateCtx {
            window: &self.handle,
            needs_inval: false,
        };
        // Note: we probably want to aggregate updates so there's only one after
        // a burst of events.
        self.root.update(&mut update_ctx, &self.data, &env);
        if ctx.base_state.needs_inval || update_ctx.needs_inval {
            self.handle.invalidate();
        }
        // TODO: process actions
        ctx.is_handled()
    }

    fn paint(&mut self, piet: &mut Piet) -> bool {
        let bc = BoxConstraints::tight(self.size);
        let env = self.root_env();
        let mut layout_ctx = LayoutCtx {};
        let size = self.root.layout(&mut layout_ctx, &bc, &self.data, &env);
        self.root.state.layout_rect = Rect::from_origin_size(Point::ORIGIN, size);
        piet.clear(BACKGROUND_COLOR);
        let mut paint_ctx = PaintCtx { render_ctx: piet };
        self.root.paint(&mut paint_ctx, &self.data, &env);
        false
    }
}

impl<T: Data> UiMain<T> {
    pub fn new(state: UiState<T>) -> UiMain<T> {
        UiMain {
            state: RefCell::new(state),
        }
    }
}

impl<T: Data + 'static> WinHandler for UiMain<T> {
    fn connect(&self, handle: &WindowHandle) {
        let mut state = self.state.borrow_mut();
        state.handle = handle.clone();
    }

    fn paint(&self, piet: &mut Piet) -> bool {
        self.state.borrow_mut().paint(piet)
    }

    fn size(&self, width: u32, height: u32) {
        let mut state = self.state.borrow_mut();
        let dpi = state.handle.get_dpi() as f64;
        let scale = 96.0 / dpi;
        state.size = Size::new(width as f64 * scale, height as f64 * scale);
    }

    fn mouse_down(&self, event: &window::MouseEvent) {
        let mut state = self.state.borrow_mut();
        // TODO: double-click detection
        let event = Event::MouseDown(event.clone());
        state.do_event(event);
    }

    fn mouse_up(&self, event: &MouseEvent) {
        let mut state = self.state.borrow_mut();
        let event = Event::MouseUp(event.clone());
        state.do_event(event);
    }

    fn mouse_move(&self, event: &MouseEvent) {
        let mut state = self.state.borrow_mut();
        let event = Event::MouseMoved(event.clone());
        state.do_event(event);
    }

    fn key_down(&self, event: KeyEvent) -> bool {
        let mut state = self.state.borrow_mut();
        state.do_event(Event::KeyDown(event))
    }

    fn key_up(&self, event: KeyEvent) {
        let mut state = self.state.borrow_mut();
        state.do_event(Event::KeyUp(event));
    }

    fn wheel(&self, delta: Vec2, mods: KeyModifiers) {
        let mut state = self.state.borrow_mut();
        let event = Event::Wheel(WheelEvent { delta, mods });
        state.do_event(event);
    }

    fn as_any(&self) -> &dyn Any {
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

impl<'a> EventCtx<'a> {
    /// Invalidate.
    ///
    /// Right now, it just invalidates the entire window, but we'll want
    /// finer grained invalidation before long.
    pub fn invalidate(&mut self) {
        self.base_state.needs_inval = true;
    }

    pub fn set_active(&mut self, active: bool) {
        self.base_state.is_active = active;
        // TODO: plumb mouse grab through to platform (through druid-shell)
    }

    pub fn is_hot(&self) -> bool {
        self.base_state.is_hot
    }

    pub fn is_active(&self) -> bool {
        self.base_state.is_active
    }

    /// Returns a reference to the current `WindowHandle`.
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
}

impl<'a> UpdateCtx<'a> {
    pub fn invalidate(&mut self) {
        self.needs_inval = true;
    }

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
