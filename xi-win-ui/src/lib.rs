// Copyright 2018 Google LLC
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

//! Simple entity-component-system based GUI.

extern crate xi_win_shell;
extern crate direct2d;
extern crate directwrite;

use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::mem;
use std::ops::Deref;

use direct2d::math::*;
use direct2d::RenderTarget;
use direct2d::render_target::GenericRenderTarget;
use direct2d::brush::SolidColorBrush;

use xi_win_shell::paint;
use xi_win_shell::win_main;
use xi_win_shell::window::{MouseButton, MouseType, WindowHandle, WinHandler};

pub mod widget;

pub use widget::Widget;

pub struct GuiMain {
    state: RefCell<GuiState>,
}

/// An identifier for widgets, scoped to a GuiMain instance. This is the
/// "entity" of the entity-component-system architecture.
pub type Id = usize;

pub struct GuiState {
    listeners: BTreeMap<Id, Vec<Box<FnMut(&Any, &mut ListenerCtx)>>>,

    /// The widget tree and associated state is split off into a separate struct
    /// so that we can use a mutable reference to it as the listener context.
    inner: ListenerCtx,
}

/// The context given to listeners.
///
/// Listeners are allowed to poke widgets and mutate the graph.
pub struct ListenerCtx {
    /// The individual widget trait objects.
    widgets: Vec<Box<Widget>>,

    /// Graph of widgets (actually a strict tree structure, so maybe should be renamed).
    graph: Graph,

    /// The state (other than widget tree) is a separate object, so that a
    /// mutable reference to it can be used as a layout context.
    c: LayoutCtx,
}

/// The context given to layout methods.
pub struct LayoutCtx {
    dwrite_factory: directwrite::Factory,

    handle: WindowHandle,

    /// Bounding box of each widget. The position is relative to the parent.
    geom: Vec<Geometry>,

    /// Queue of events to distribute to listeners
    event_q: Vec<(Id, Box<Any>)>,
}

#[derive(Default)]
struct Graph {
    root: Id,
    children: Vec<Vec<Id>>,
    parent: Vec<Id>,
}

#[derive(Default, Clone, Copy)]
pub struct Geometry {
    // Maybe PointF is a better type, then we could use the math from direct2d?
    pub pos: (f32, f32),
    pub size: (f32, f32),
}

pub struct BoxConstraints {
    min_width: f32,
    max_width: f32,
    min_height: f32,
    max_height: f32,
}

pub enum LayoutResult {
    Size((f32, f32)),
    RequestChild(Id, BoxConstraints),
}

// Contexts for widget methods.

/// Context given to handlers.
pub struct HandlerCtx<'a> {
    /// The id of the node sending the event
    id: Id,

    c: &'a mut LayoutCtx,
}

pub struct PaintCtx<'a, 'b: 'a>  {
    inner: &'a mut paint::PaintCtx<'b>,
    dwrite_factory: &'a directwrite::Factory,
}

/// A command for exiting. TODO: move commands entirely to client.
pub const COMMAND_EXIT: u32 = 0x100;

impl Geometry {
    fn offset(&self, offset: (f32, f32)) -> Geometry {
        Geometry {
            pos: (self.pos.0 + offset.0, self.pos.1 + offset.1),
            size: self.size
        }
    }
}

impl GuiMain {
    pub fn new(state: GuiState) -> GuiMain {
        GuiMain { state: RefCell::new(state) }
    }
}

impl GuiState {
    pub fn new() -> GuiState {
        GuiState {
            listeners: Default::default(),
            inner: ListenerCtx {
                widgets: Vec::new(),
                graph: Default::default(),
                c: LayoutCtx {
                    dwrite_factory: directwrite::Factory::new().unwrap(),
                    geom: Vec::new(),
                    handle: Default::default(),
                    event_q: Vec::new(),
                }
            }
        }
    }

    /// Add a listener that expects a specific type.
    pub fn add_listener<A, F>(&mut self, node: Id, mut f: F)
        where A: Any + Copy, F: FnMut(A, &mut ListenerCtx) + 'static
    {
        let wrapper: Box<FnMut(&Any, &mut ListenerCtx)> = Box::new(move |a, ctx| {
            if let Some(arg) = a.downcast_ref() {
                f(*arg, ctx)
            } else {
                println!("type mismatch in listener arg");
            }
        });
        self.listeners.entry(node).or_insert(Vec::new()).push(wrapper);
    }

    fn mouse(&mut self, x: f32, y: f32, mods: u32, which: MouseButton, ty: MouseType) {
        mouse_rec(&mut self.inner.widgets, &self.inner.graph,
            x, y, mods, which, ty,
            &mut HandlerCtx {
                id: self.inner.graph.root,
                c: &mut self.inner.c,
            }
        );
        self.dispatch_events();
    }

    fn dispatch_events(&mut self) {
        let event_q = mem::replace(&mut self.c.event_q, Vec::new());
        for (id, event) in event_q {
            if let Some(listeners) = self.listeners.get_mut(&id) {
                for listener in listeners {
                    listener(event.deref(), &mut self.inner);
                }
            }
        }
    }
}

// Do pre-order traversal on graph, painting each node in turn.
//
// Implemented as a recursion, but we could use an explicit queue instead.
fn paint_rec(widgets: &mut [Box<Widget>], graph: &Graph, geom: &[Geometry],
    paint_ctx: &mut PaintCtx, node: Id, pos: (f32, f32))
{
    let g = geom[node].offset(pos);
    widgets[node].paint(paint_ctx, &g);
    for child in graph.children[node].clone() {
        paint_rec(widgets, graph, geom, paint_ctx, child, g.pos);
    }
}

fn layout_rec(widgets: &mut [Box<Widget>], ctx: &mut LayoutCtx, graph: &Graph,
    bc: &BoxConstraints, node: Id) -> (f32, f32)
{
    let mut size = None;
    loop {
        let layout_res = widgets[node].layout(bc, &graph.children[node], size, ctx);
        match layout_res {
            LayoutResult::Size(size) => {
                ctx.geom[node].size = size;
                return size;
            }
            LayoutResult::RequestChild(child, child_bc) => {
                size = Some(layout_rec(widgets, ctx, graph, &child_bc, child));
            }
        }
    }
}

fn clamp(val: f32, min: f32, max: f32) -> f32 {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}

impl Deref for GuiState {
    type Target = ListenerCtx;

    fn deref(&self) -> &ListenerCtx {
        &self.inner
    }
}

impl ::std::ops::DerefMut for GuiState {
    fn deref_mut(&mut self) -> &mut ListenerCtx {
        &mut self.inner
    }
}

impl ListenerCtx {
    /// Send an arbitrary payload to a widget. The type and interpretation of the
    /// payload depends on the specific target widget.
    pub fn poke<A: Any>(&mut self, node: Id, payload: &mut A) -> bool {
        let mut ctx = HandlerCtx {
            id: node,
            c: &mut self.c,
        };
        self.widgets[node].poke(payload, &mut ctx)
    }

    /// Put a widget in the graph and add its children. Returns newly allocated
    /// id for the node.
    pub fn add<W>(&mut self, widget: W, children: &[Id]) -> Id
        where W: Widget + 'static
    {
        let id = self.graph.alloc_node();
        self.widgets.push(Box::new(widget));
        self.c.geom.push(Default::default());
        for &child in children {
            self.graph.append_child(id, child);
        }
        id
    }

    pub fn set_root(&mut self, root: Id) {
        self.graph.root = root;
    }

    // The following methods are really GuiState methods, but don't need access to listeners
    // so are more concise to implement here.

    fn paint(&mut self, paint_ctx: &mut paint::PaintCtx, root: Id) {
        let mut paint_ctx = PaintCtx {
            inner: paint_ctx,
            dwrite_factory: &self.c.dwrite_factory,
        };
        paint_rec(&mut self.widgets, &self.graph, &self.c.geom,
            &mut paint_ctx, root, (0.0, 0.0));
    }

    fn layout(&mut self, bc: &BoxConstraints, root: Id) {
        layout_rec(&mut self.widgets, &mut self.c, &self.graph, bc, root);
    }
}

impl BoxConstraints {
    pub fn tight(size: (f32, f32)) -> BoxConstraints {
        BoxConstraints {
            min_width: size.0,
            max_width: size.0,
            min_height: size.1,
            max_height: size.1,
        }
    }

    pub fn constrain(&self, size: (f32, f32)) -> (f32, f32) {
        (clamp(size.0, self.min_width, self.max_width),
            clamp(size.1, self.min_height, self.max_height))
    }
}

impl LayoutCtx {
    pub fn dwrite_factory(&self) -> &directwrite::Factory {
        &self.dwrite_factory
    }

    pub fn position_child(&mut self, child: Id, pos: (f32, f32)) {
        self.geom[child].pos = pos;
    }

    pub fn get_child_size(&self, child: Id) -> (f32, f32) {
        self.geom[child].size
    }
}

fn mouse_rec(widgets: &mut [Box<Widget>], graph: &Graph,
    x: f32, y: f32, mods: u32, which: MouseButton, ty: MouseType, ctx: &mut HandlerCtx)
    -> bool
{
    let node = ctx.id;
    let g = ctx.c.geom[node];
    let x = x - g.pos.0;
    let y = y - g.pos.1;
    let mut handled = false;
    if x >= 0.0 && y >= 0.0 && x < g.size.0 && y < g.size.1 {
        handled = widgets[node].mouse(x, y, mods, which, ty, ctx);
        for child in graph.children[node].iter().rev() {
            if handled {
                break;
            }
            ctx.id = *child;
            handled = mouse_rec(widgets, graph, x, y, mods, which, ty, ctx);
        }
    }
    handled
}

impl<'a> HandlerCtx<'a> {
    pub fn invalidate(&self) {
        self.c.handle.invalidate();
    }

    // Send an event, to be handled by listeners.
    pub fn send_event<A: Any>(&mut self, a: A) {
        let id = self.id;
        self.c.event_q.push((id, Box::new(a)));
    }
}

impl<'a, 'b> PaintCtx<'a, 'b> {
    pub fn dwrite_factory(&self) -> &directwrite::Factory {
        self.dwrite_factory
    }

    pub fn render_target(&mut self) -> &mut GenericRenderTarget {
        self.inner.render_target()
    }
}

impl WinHandler for GuiMain {
    fn connect(&self, handle: &WindowHandle) {
        self.state.borrow_mut().c.handle = handle.clone();
    }

    fn paint(&self, paint_ctx: &mut paint::PaintCtx) -> bool {
        let size;
        {
            let rt = paint_ctx.render_target();
            size = rt.get_size();
            let rect = RectF::from((0.0, 0.0, size.width, size.height));
            let bg = SolidColorBrush::create(rt).with_color(0x272822).build().unwrap();
            rt.fill_rectangle(rect, &bg);
        }
        let mut state = self.state.borrow_mut();
        let root = state.graph.root;
        let bc = BoxConstraints::tight((size.width, size.height));
        // TODO: be lazier about relayout
        state.layout(&bc, root);
        state.paint(paint_ctx, root);
        false
    }

    fn command(&self, id: u32) {
        // TODO: plumb through to client
        match id {
            COMMAND_EXIT => self.state.borrow().c.handle.close(),
            _ => println!("unexpected id {}", id),
        }
    }

    fn char(&self, ch: u32, mods: u32) {
        println!("got char 0x{:x} {:02x}", ch, mods);
    }

    fn keydown(&self, vk_code: i32, mods: u32) -> bool {
        println!("got key code 0x{:x} {:02x}", vk_code, mods);
        false
    }

    fn mouse_wheel(&self, delta: i32, mods: u32) {
        println!("mouse_wheel {} {:02x}", delta, mods);
    }

    fn mouse_hwheel(&self, delta: i32, mods: u32) {
        println!("mouse_hwheel {} {:02x}", delta, mods);
    }

    fn mouse_move(&self, x: i32, y: i32, mods: u32) {
        println!("mouse_move ({}, {}) {:02x}", x, y, mods);
    }

    fn mouse(&self, x: i32, y: i32, mods: u32, button: MouseButton, ty: MouseType) {
        println!("mouse ({}, {}) {:02x} {:?} {:?}", x, y, mods, button, ty);
        let mut state = self.state.borrow_mut();
        let (x, y) = state.c.handle.pixels_to_px_xy(x, y);
        // TODO: detect multiple clicks and pass that down
        state.mouse(x, y, mods, button, ty);
    }

    fn destroy(&self) {
        win_main::request_quit();
    }

    fn as_any(&self) -> &Any { self }
}

impl Graph {
    pub fn alloc_node(&mut self) -> Id {
        let id = self.children.len();
        self.children.push(vec![]);
        self.parent.push(id);
        id
    }

    pub fn append_child(&mut self, parent: Id, child: Id) {
        self.children[parent].push(child);
        self.parent[child] = parent;
    }
}
