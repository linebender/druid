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

use druid::kurbo::{Point, Rect, Size};
use druid::piet::{Color, FillRule, RenderContext};

use druid::shell::{runloop, WindowBuilder};
use druid::widget::{ScrollEvent, Widget};
use druid::{
    BoxConstraints, HandlerCtx, Id, LayoutCtx, LayoutResult, PaintCtx, Ui, UiMain, UiState,
};

const BG_COLOR: Color = Color::rgb24(0xfb_f8_ef);
const MOUSE_BOX_COLOR: Color = Color::rgb24(0xb8_32_5a);

struct FooWidget {
    pos: Point,
    size: Size,
}

impl Widget for FooWidget {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Rect) {
        paint_ctx.render_ctx.clear(BG_COLOR);

        let fg = paint_ctx.render_ctx.solid_brush(MOUSE_BOX_COLOR);
        let pos = self.pos + geom.origin().to_vec2();
        paint_ctx.render_ctx.fill(
            Rect::from_origin_size(pos, self.size),
            &fg,
            FillRule::NonZero,
        );
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<Size>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        LayoutResult::Size(bc.constrain((100.0, 100.0)))
    }

    fn scroll(&mut self, event: &ScrollEvent, ctx: &mut HandlerCtx) {
        self.size.width += event.dx as f64;
        self.size.height += event.dy as f64;
        ctx.invalidate();
    }

    fn mouse_moved(&mut self, pos: Point, ctx: &mut HandlerCtx) {
        self.pos = pos;
        ctx.invalidate();
    }
}

impl FooWidget {
    fn new() -> Self {
        Self {
            pos: Point::new(10.0, 10.0),
            size: Size::new(40.0, 40.0),
        }
    }

    fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}

fn main() {
    druid_shell::init();

    let mut run_loop = runloop::RunLoop::new();
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
