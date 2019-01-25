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

extern crate direct2d;
extern crate directwrite;
extern crate druid;
extern crate druid_win_shell;

use direct2d::brush::SolidColorBrush;
use direct2d::RenderTarget;

use druid_win_shell::win_main;
use druid_win_shell::window::WindowBuilder;

use druid::{Ui, UiMain, UiState};

use druid::widget::Widget;
use druid::{BoxConstraints, Geometry, LayoutResult};
use druid::{HandlerCtx, Id, LayoutCtx, MouseEvent, PaintCtx};

/// A custom widget with animations.
struct AnimWidget(f32);

impl Widget for AnimWidget {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        let rt = paint_ctx.render_target();
        let fg = SolidColorBrush::create(rt)
            .with_color(0xf0f0ea)
            .build()
            .unwrap();
        let (x, y) = geom.pos;
        rt.draw_line(
            (x, y),
            (x + geom.size.0, y + self.0 * geom.size.1),
            &fg,
            1.0,
            None,
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

    fn anim_frame(&mut self, interval: u64, ctx: &mut HandlerCtx) {
        println!("anim frame, interval={}", interval);
        if self.0 > 0.0 {
            ctx.request_anim_frame();
            self.0 = (self.0 - 1e-9 * (interval as f32)).max(0.0);
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
    druid_win_shell::init();

    let mut run_loop = win_main::RunLoop::new();
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
