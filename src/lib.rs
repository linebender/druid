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

mod value;

use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::time::Instant;

use kurbo::{Point, Rect, Size, Vec2};
use piet::{Color, Piet, RenderContext};

use druid_shell::application::Application;
pub use druid_shell::dialog::{FileDialogOptions, FileDialogType};
pub use druid_shell::keyboard::{KeyCode, KeyEvent, KeyModifiers};
use druid_shell::platform::IdleHandle;
use druid_shell::window::{self, MouseType, WinHandler, WindowHandle};

use value::{Delta, KeyPath, PathEl, Value};

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

#[derive(Default)]
pub struct BaseState {
    layout_rect: Rect,

    // This should become an invalidation rect.
    needs_inval: bool,
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

pub struct LayoutCtx {
}

pub struct EventCtx<'a> {
    base_state: &'a mut BaseState,
}

pub enum Event {}

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

    fn paint(&mut self, paint_ctx: &mut PaintCtx, env: &Env) {
        self.inner.paint(paint_ctx, &self.state, env);
    }

    pub fn layout(&mut self, layout_ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        self.inner.layout(layout_ctx, bc, env)
    }

    /// Propagate an event.
    pub fn event(&mut self, event: &Event, ctx: &mut EventCtx, env: &Env) -> Option<Action> {
        let mut child_ctx = EventCtx {
            base_state: &mut self.state,
        };
        child_ctx.base_state.needs_inval = false;
        let action = self.inner.event(event, &mut child_ctx, env);
        ctx.base_state.needs_inval |= child_ctx.base_state.needs_inval;
        action
    }
}

// Consider putting the `'static` bound on the main impl.
impl<W: WidgetInner + 'static> WidgetBase<W> {
    pub fn boxed(self) -> WidgetBase<Box<dyn WidgetInner>> {
        WidgetBase {
            state: self.state,
            inner: Box::new(self.inner),
        }
    }
}

impl UiState {
    pub fn new(root: WidgetBase<Box<dyn WidgetInner>>) -> UiState {
        UiState {
            root,
            handle: Default::default(),
            size: Default::default(),
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
        let env = Default::default();
        let mut layout_ctx = LayoutCtx {};
        let _ = state.root.layout(&mut layout_ctx, &bc, &env);
        piet.clear(BACKGROUND_COLOR);
        let mut paint_ctx = PaintCtx { render_ctx: piet };
        state.root.paint(&mut paint_ctx, &env);
        false
    }

    fn size(&self, width: u32, height: u32) {
        let mut state = self.state.borrow_mut();
        let dpi = state.handle.get_dpi() as f64;
        let scale = 96.0 / dpi;
        state.size = Size::new(width as f64 * scale, height as f64 * scale);
    }

    fn as_any(&self) -> &dyn Any {
        self
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
    pub fn join(&self, path: &[PathEl]) -> Env {
        let path = [&self.path, path].concat();
        Env {
            value: self.value.clone(),
            path,
        }
    }

    pub fn get_data(&self) -> &Value {
        self.value.access(&self.path).unwrap()
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
}
