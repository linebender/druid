// Copyright 2019 The xi-editor Authors.
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

use druid::shell::{runloop, WindowBuilder};
use druid::widget::{ActionWrapper, Button, Column, DynLabel, Padding};

use druid::{
    Data, DruidHandler, Env, Event, EventCtxRoot, KeyCode, LayoutCtxRoot, PaintCtxRoot, RootWidget,
    UpdateCtxRoot, Widget, WindowId, WindowSet,
};

pub struct SharedWindow<T: Data> {
    windows: WindowSet<T>,
}

impl<T: Data> SharedWindow<T> {
    pub fn new(root: impl Widget<T> + 'static, id: WindowId) -> SharedWindow<T> {
        SharedWindow {
            windows: WindowSet::new(root, id),
        }
    }
}

impl RootWidget<u32> for SharedWindow<u32> {
    fn event(&mut self, event: &Event, ctx: &mut EventCtxRoot, data: &mut u32, env: &Env) {
        match event {
            Event::KeyDown(e) => {
                // TODO: I know this is not the best practice to match
                if e.mods.ctrl && e.key_code == KeyCode::KeyN {
                    let mut builder = WindowBuilder::new();
                    builder.set_title("Additional window");
                    let (id, window_pod) = ctx.new_win(builder, make_ui());
                    self.windows.add_window(id, window_pod);
                    println!("new window {:?}", id);
                }
            }
            _ => (),
        }
        self.windows.event(event, ctx, data, env);
    }

    /// Propagate a data update to all windows.
    fn update(&mut self, ctx: &mut UpdateCtxRoot, data: &u32, env: &Env) {
        self.windows.update(ctx, data, env);
    }

    /// Propagate layout to a child window.
    ///
    /// The case for this method is weak; it could be subsumed into the `paint` method.
    fn layout(&mut self, ctx: &mut LayoutCtxRoot, data: &u32, env: &Env) {
        self.windows.layout(ctx, data, env);
    }

    /// Paint a child window's appearance.
    fn paint(&mut self, paint_ctx: &mut PaintCtxRoot, data: &u32, env: &Env) {
        self.windows.paint(paint_ctx, data, env);
    }
}

fn make_ui() -> impl Widget<u32> {
    let mut col = Column::new();
    let label = DynLabel::new(|data: &u32, _env| format!("value: {}", data));
    let button = Button::new("increment");
    col.add_child(Padding::uniform(5.0, label), 1.0);
    col.add_child(Padding::uniform(5.0, button), 1.0);
    ActionWrapper::new(col, |data: &mut u32, _env| *data += 1)
}

fn main() {
    druid::shell::init();

    let mut run_loop = runloop::RunLoop::new();
    let id = WindowId::new();
    let mut builder = WindowBuilder::new();
    let root = make_ui();
    let shared = SharedWindow::new(root, id);
    let handler = DruidHandler::new(shared, 0u32, id);
    builder.set_title("Hello example");
    builder.set_handler(Box::new(handler));
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
