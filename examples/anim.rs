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

//! Example of animation frames.

use druid::kurbo::{Line, Rect, Size, Vec2};
use druid::piet::{Color, RenderContext};
use druid::shell::{runloop, WindowBuilder};

use druid::widget::Widget;
use druid::{
    BoxConstraints, HandlerCtx, Id, LayoutCtx, LayoutResult, MouseEvent, PaintCtx, Ui, UiMain,
    UiState,
};

const BG_COLOR: Color = Color::rgb24(0xfb_f8_ef);

/// A custom widget with animations.
struct AnimWidget(f64);

impl Widget for AnimWidget {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Rect) {
        let fg = paint_ctx.render_ctx.solid_brush(BG_COLOR);
        paint_ctx.render_ctx.stroke(
            Line::new(
                geom.origin(),
                geom.origin() + Vec2::new(geom.width(), self.0 * geom.height()),
            ),
            &fg,
            1.0,
            None,
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

    fn anim_frame(&mut self, interval: u64, ctx: &mut HandlerCtx) {
        println!("anim frame, interval={}", interval);
        if self.0 > 0.0 {
            ctx.request_anim_frame();
            self.0 = (self.0 - 1e-9 * (interval as f64)).max(0.0);
        }
        ctx.invalidate();
    }

    fn mouse(&mut self, event: &MouseEvent, ctx: &mut HandlerCtx) -> bool {
        if event.count > 0 {
            self.0 = 1.0;
            ctx.request_anim_frame();
        }
        true
    }
}

impl AnimWidget {
    fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}

fn main() {
    druid::shell::init();

    let mut run_loop = runloop::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();
    let anim = AnimWidget(1.0).ui(&mut state);
    state.set_root(anim);
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Animation example");
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
