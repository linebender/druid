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

//! Sample GUI app.

extern crate druid;
extern crate druid_shell;
extern crate kurbo;
extern crate piet;

use kurbo::Rect;
use piet::FillRule;
use piet::RenderContext;

use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;

use druid::{Ui, UiMain, UiState};

use druid::widget::{Widget, ScrollEvent};
use druid::HandlerCtx;
use druid::{BoxConstraints, Geometry, LayoutResult};
use druid::{Id, LayoutCtx, PaintCtx};

struct FooWidget {
    pos: (f64, f64),
    size: (f64, f64),
}

impl Widget for FooWidget {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        paint_ctx.render_ctx.clear(0xfbf8ef);

        let fg = paint_ctx.render_ctx.solid_brush(0xb8325aff).unwrap();
        let (x, y) = geom.pos;
        let (x, y) = (x as f64, y as f64);
        let (x, y) = (x + self.pos.0, y + self.pos.1);

        paint_ctx.render_ctx.fill(
            Rect::new(
                x,
                y,
                x + self.size.0,
                y + self.size.1,
            ),
            &fg,
            FillRule::NonZero,
        );
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<(f32, f32)>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        LayoutResult::Size(bc.constrain((100.0, 100.0)))
    }

    fn scroll(&mut self, event: &ScrollEvent, ctx: &mut HandlerCtx) {
        self.size.0 += event.dx as f64;
        self.size.1 += event.dy as f64;
        ctx.invalidate();
    }

    fn mouse_moved(&mut self, x: f32, y: f32, ctx: &mut HandlerCtx) {
        self.pos = (x as f64, y as f64);
        ctx.invalidate();
    }
}

impl FooWidget {
    fn new() -> Self {
        Self {
            pos: (10.0, 10.0),
            size: (40.0, 40.0),
        }
    }

    fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}

fn main() {
    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();
    let foo = FooWidget::new().ui(&mut state);
    state.set_root(foo);
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Mouse example");
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
