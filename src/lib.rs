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

pub use druid_shell::{kurbo, piet};

pub mod widget;

mod event;
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
use druid_shell::window::{self, MouseType, WinHandler, WindowHandle};

use event::{Event, MouseEvent};
use value::{Delta, KeyPath, PathEl, PathFragment, Value};

const BACKGROUND_COLOR: Color = Color::rgb24(0x27_28_22);

pub struct UiMain {
    state: RefCell<UiState>,
}

pub struct UiState {
    root: WidgetBase<Box<dyn WidgetInner>>,
    // Following fields might move to a separate struct so there's access
    // from contexts.
    handle: WindowHandle,
    size: Size,
}

pub struct WidgetBase<W: WidgetInner> {
    state: BaseState,
    inner: W,
}

/// Convenience type for dynamic boxed widget.
pub type BoxedWidget = WidgetBase<Box<dyn WidgetInner>>;

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

pub trait WidgetInner {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, env: &Env);

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size;

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, env: &Env) -> Option<Action>;
}

impl WidgetInner for Box<dyn WidgetInner> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, env: &Env) {
        self.deref_mut().paint(paint_ctx, base_state, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        self.deref_mut().layout(ctx, bc, env)
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, env: &Env) -> Option<Action> {
        self.deref_mut().event(event, ctx, env)
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
    base_state: &'a mut BaseState,
    had_active: bool,
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

impl<W: WidgetInner> WidgetBase<W> {
    pub fn new(inner: W) -> WidgetBase<W> {
        WidgetBase {
            state: Default::default(),
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

    pub fn paint(&mut self, paint_ctx: &mut PaintCtx, env: &Env, frag: impl PathFragment) {
        let env = env.join(frag);
        self.inner.paint(paint_ctx, &self.state, &env);
    }

    /// Paint the widget, translating it by the origin of its layout rectangle.
    // Discussion: should this be `paint` and the other `paint_raw`?
    pub fn paint_with_offset(
        &mut self,
        paint_ctx: &mut PaintCtx,
        env: &Env,
        frag: impl PathFragment,
    ) {
        if let Err(e) = paint_ctx.render_ctx.save() {
            eprintln!("error saving render context: {:?}", e);
            return;
        }
        paint_ctx
            .render_ctx
            .transform(Affine::translate(self.state.layout_rect.origin().to_vec2()));
        self.paint(paint_ctx, env, frag);
        if let Err(e) = paint_ctx.render_ctx.restore() {
            eprintln!("error restoring render context: {:?}", e);
        }
    }

    pub fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        env: &Env,
        frag: impl PathFragment,
    ) -> Size {
        let env = env.join(frag);
        self.inner.layout(layout_ctx, bc, &env)
    }

    /// Propagate an event.
    pub fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        env: &Env,
        frag: impl PathFragment,
    ) -> Option<Action> {
        if !event.recurse() {
            // This function is called by containers to propagate an event from
            // containers to children. Non-recurse events will be invoked directly
            // from other points in the library.
            return None;
        }
        let env = env.join(frag);
        let had_active = self.state.has_active;
        let mut child_ctx = EventCtx {
            base_state: &mut self.state,
            had_active,
        };
        // Note: could also represent this as `Option<Event>`.
        let mut recurse = true;
        let child_event = match event {
            Event::Mouse(mouse_event) => {
                let rect = child_ctx.base_state.layout_rect;
                recurse = had_active || !ctx.had_active && rect.winding(mouse_event.pos) != 0;
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos -= rect.origin().to_vec2();
                Event::Mouse(mouse_event)
            }
            Event::MouseMoved(point) => {
                let rect = child_ctx.base_state.layout_rect;
                let had_hot = child_ctx.base_state.is_hot;
                child_ctx.base_state.is_hot = rect.winding(*point) != 0;
                recurse = had_active || had_hot || child_ctx.base_state.is_hot;
                let point = *point - rect.origin().to_vec2();
                Event::MouseMoved(point)
            }
            Event::HotChanged(is_hot) => Event::HotChanged(*is_hot),
        };
        child_ctx.base_state.needs_inval = false;
        let action = if recurse {
            child_ctx.base_state.has_active = false;
            let action = self.inner.event(&child_event, &mut child_ctx, &env);
            child_ctx.base_state.has_active |= child_ctx.base_state.is_active;
            action
        } else {
            None
        };
        ctx.base_state.needs_inval |= child_ctx.base_state.needs_inval;
        ctx.base_state.is_hot |= child_ctx.base_state.is_hot;
        ctx.base_state.has_active |= child_ctx.base_state.has_active;
        action
    }
}

// Consider putting the `'static` bound on the main impl.
impl<W: WidgetInner + 'static> WidgetBase<W> {
    pub fn boxed(self) -> BoxedWidget {
        WidgetBase {
            state: self.state,
            inner: Box::new(self.inner),
        }
    }
}

// Convenience method for conversion to boxed widgets.
impl<W: WidgetInner + 'static> From<W> for BoxedWidget {
    fn from(w: W) -> BoxedWidget {
        WidgetBase::new(w).boxed()
    }
}

impl UiState {
    pub fn new(root: impl Into<BoxedWidget>) -> UiState {
        UiState {
            root: root.into(),
            handle: Default::default(),
            size: Default::default(),
        }
    }

    fn root_env(&self) -> Env {
        Default::default()
    }

    fn do_event(&mut self, event: Event) {
        // should there be a root base state persisting in the ui state instead?
        let mut base_state = Default::default();
        let mut ctx = EventCtx {
            base_state: &mut base_state,
            had_active: self.root.state.has_active,
        };
        let env = self.root_env();
        let action = self.root.event(&event, &mut ctx, &env, ());
        if ctx.base_state.needs_inval {
            self.handle.invalidate();
        }
        // TODO: process actions
        if let Some(action) = action {
            println!("action: {:?}", action);
        }
    }
}

impl UiMain {
    pub fn new(state: UiState) -> UiMain {
        UiMain {
            state: RefCell::new(state),
        }
    }
}

impl WinHandler for UiMain {
    fn connect(&self, handle: &WindowHandle) {
        let mut state = self.state.borrow_mut();
        state.handle = handle.clone();
    }

    fn paint(&self, piet: &mut Piet) -> bool {
        let mut state = self.state.borrow_mut();
        let bc = BoxConstraints::tight(state.size);
        let env = state.root_env();
        let mut layout_ctx = LayoutCtx {};
        let size = state.root.layout(&mut layout_ctx, &bc, &env, ());
        state.root.state.layout_rect = Rect::from_origin_size(Point::ORIGIN, size);
        piet.clear(BACKGROUND_COLOR);
        let mut paint_ctx = PaintCtx { render_ctx: piet };
        state.root.paint(&mut paint_ctx, &env, ());
        false
    }

    fn size(&self, width: u32, height: u32) {
        let mut state = self.state.borrow_mut();
        let dpi = state.handle.get_dpi() as f64;
        let scale = 96.0 / dpi;
        state.size = Size::new(width as f64 * scale, height as f64 * scale);
    }

    fn mouse(&self, event: &window::MouseEvent) {
        let mut state = self.state.borrow_mut();
        let (x, y) = state.handle.pixels_to_px_xy(event.x, event.y);
        println!("mouse {:?} -> ({}, {})", event, x, y);
        let pos = Point::new(x as f64, y as f64);
        // TODO: double-click detection
        let count = if event.ty == MouseType::Down { 1 } else { 0 };
        let event = Event::Mouse(MouseEvent {
            pos,
            mods: event.mods,
            which: event.which,
            count,
        });
        state.do_event(event);
    }

    fn mouse_move(&self, x: i32, y: i32, _mods: u32) {
        let mut state = self.state.borrow_mut();
        let (x, y) = state.handle.pixels_to_px_xy(x, y);
        let pos = Point::new(x as f64, y as f64);
        let event = Event::MouseMoved(pos);
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
}

impl Action {
    /// Make an action from a string.
    ///
    /// Note: this is something of a placeholder and will change.
    pub fn from_str(s: impl Into<String>) -> Action {
        Action { text: s.into() }
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
