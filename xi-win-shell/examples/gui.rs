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

//! Sketch of entity-component based GUI.

extern crate xi_win_shell;
extern crate direct2d;

use std::any::Any;
use std::cell::RefCell;

use direct2d::math::*;
use direct2d::RenderTarget;
use direct2d::brush::SolidColorBrush;

use xi_win_shell::menu::Menu;
use xi_win_shell::paint::PaintCtx;
use xi_win_shell::win_main;
use xi_win_shell::window::{MouseButton, MouseType, WindowBuilder, WindowHandle, WinHandler};

#[derive(Default)]
struct GuiMain {
    state: RefCell<GuiState>,
}

type Id = usize;

#[derive(Default)]
struct GuiState {
    handle: WindowHandle,
    widgets: Vec<Box<Widget>>,

    // The position in the geometry is relative to the parent.
    geom: Vec<Geometry>,
    graph: Graph,
}

#[derive(Default)]
struct Graph {
    root: Id,
    children: Vec<Vec<Id>>,
    parent: Vec<Id>,
}

#[derive(Default)]
struct Geometry {
    // Maybe PointF is a better type, then we could use the math from direct2d?
    pos: (f32, f32),
    size: (f32, f32),
}

struct BoxConstraints {
    min_width: f32,
    max_width: f32,
    min_height: f32,
    max_height: f32,
}

enum LayoutResult {
    Size((f32, f32)),
    RequestChild(Id, BoxConstraints),
}

struct LayoutCtx<'a>(&'a mut [Geometry]);

trait Widget {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry);

    /// `size` is the size of the child previously requested by a RequestChild return.
    fn layout(&mut self, bc: &BoxConstraints, children: &[Id], size: Option<(f32, f32)>,
        ctx: &mut LayoutCtx) -> LayoutResult;

    /// An `escape hatch` of sorts for accessing widget state beyond the widget
    /// methods. Returns true if it is handled.
    fn poke(&mut self, payload: &mut Any, ctx: &mut PokeCtx) -> bool { false }
}

struct PokeCtx;

struct FooWidget;

#[derive(Default)]
struct Row {
    // layout continuation state
    ix: usize,
    width_per_flex: f32,
    height: f32,
}

impl Geometry {
    fn offset(&self, offset: (f32, f32)) -> Geometry {
        Geometry {
            pos: (self.pos.0 + offset.0, self.pos.1 + offset.1),
            size: self.size
        }
    }
}

// Functions like this suggest a proper size newtype.
fn add_pos(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    (a.0 + b.0, a.1 + b.1)
}

impl Widget for FooWidget {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        let rt = paint_ctx.render_target();
        let fg = SolidColorBrush::create(rt).with_color(0xf0f0ea).build().unwrap();
        let (x, y) = geom.pos;
        rt.draw_line((x + 10.0, y + 50.0), (x + 90.0, y + 90.0),
                &fg, 1.0, None);
    }

    fn layout(&mut self, _bc: &BoxConstraints, _children: &[Id], _size: Option<(f32, f32)>,
        _ctx: &mut LayoutCtx) -> LayoutResult
    {
        LayoutResult::Size((100.0, 100.0))
    }
}

impl GuiState {
    pub fn instantiate_widget<W>(&mut self, widget: W, children: &[Id]) -> Id
        where W: Widget + 'static
    {
        let id = self.graph.alloc_node();
        self.widgets.push(Box::new(widget));
        self.geom.push(Default::default());
        for &child in children {
            self.graph.append_child(id, child);
        }
        id
    }

    pub fn set_root(&mut self, root: Id) {
        self.graph.root = root;
    }

    // Do pre-order traversal on graph, painting each node in turn.
    //
    // Implemented as a recursion, but we could use an explicit queue instead.
    fn paint_rec(&mut self, paint_ctx: &mut PaintCtx, node: Id, pos: (f32, f32)) {
        let geom = self.geom[node].offset(pos);
        self.widgets[node].paint(paint_ctx, &geom);
        // Note: we could eliminate the clone here by carrying widgets as a mut ref,
        // and the graph as a non-mut ref.
        for child in self.graph.children[node].clone() {
            self.paint_rec(paint_ctx, child, geom.pos);
        }
    }

    fn layout(&mut self, bc: &BoxConstraints, root: Id) {
        layout_rec(&mut self.widgets, &mut self.geom, &self.graph, bc, root);
    }
}

fn layout_rec(widgets: &mut [Box<Widget>], geom: &mut [Geometry], graph: &Graph,
    bc: &BoxConstraints, node: Id) -> (f32, f32)
{
    let mut size = None;
    loop {
        let layout_res = widgets[node].layout(bc, &graph.children[node], size,
            &mut LayoutCtx(geom));
        match layout_res {
            LayoutResult::Size(size) => {
                geom[node].size = size;
                return size;
            }
            LayoutResult::RequestChild(child, child_bc) => {
                size = Some(layout_rec(widgets, geom, graph, &child_bc, child));
            }
        }
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
}

impl<'a> LayoutCtx<'a> {
    pub fn position_child(&mut self, child: Id, pos: (f32, f32)) {
        self.0[child].pos = pos;
    }

    pub fn get_child_size(&self, child: Id) -> (f32, f32) {
        self.0[child].size
    }
}

impl Widget for Row {
    // Maybe there should be a no-op default method, for containers in general?
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {        
    }

    fn layout(&mut self, bc: &BoxConstraints, children: &[Id], size: Option<(f32, f32)>,
        ctx: &mut LayoutCtx) -> LayoutResult
    {
        if let Some(size) = size {
            if size.1 > self.height {
                self.height = size.1;
            }
            self.ix += 1;
            if self.ix == children.len() {
                // measured all children
                let mut x = 0.0;
                for &child in children {
                    // top-align, could do center etc. based on child height
                    ctx.position_child(child, (x, 0.0));
                    x += self.width_per_flex;
                }
                return LayoutResult::Size((bc.max_width, self.height));
            }
        } else {
            if children.is_empty() {
                return LayoutResult::Size((bc.min_width, bc.min_height));
            }
            self.ix = 0;
            self.height = bc.min_height;
            self.width_per_flex = bc.max_width / (children.len() as f32);
        }
        let child_bc = BoxConstraints {
            min_width: self.width_per_flex,
            max_width: self.width_per_flex,
            min_height: bc.min_height,
            max_height: bc.max_height,
        };
        LayoutResult::RequestChild(self.ix, child_bc)
    }
}

impl WinHandler for GuiMain {
    fn connect(&self, handle: &WindowHandle) {
        self.state.borrow_mut().handle = handle.clone();
    }

    fn paint(&self, paint_ctx: &mut PaintCtx) -> bool {
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
        state.paint_rec(paint_ctx, root, (0.0, 0.0));
        false
    }

    fn command(&self, id: u32) {
        match id {
            0x100 => self.state.borrow().handle.close(),
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
        println!("mouse_move ({}, {}) {:02x} {:?} {:?}", x, y, mods, button, ty);
    }

    fn destroy(&self) {
        win_main::request_quit();
    }

    fn as_any(&self) -> &Any { self }
}

fn main() {
    xi_win_shell::init();

    let mut file_menu = Menu::new();
    file_menu.add_item(0x100, "E&xit");
    let mut menubar = Menu::new();
    menubar.add_dropdown(file_menu, "&File");

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = GuiState::default();
    let foo1 = state.instantiate_widget(FooWidget, &[]);
    let foo2 = state.instantiate_widget(FooWidget, &[]);
    let root = state.instantiate_widget(Row::default(), &[foo1, foo2]);
    state.set_root(root);
    builder.set_handler(Box::new(GuiMain { state: RefCell::new(state) }));
    builder.set_title("Hello example");
    builder.set_menu(menubar);
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
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
