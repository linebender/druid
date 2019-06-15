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

extern crate druid;
extern crate druid_shell;
extern crate kurbo;
extern crate piet;

use kurbo::Circle;
use piet::{FillRule, RenderContext};

use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;

use druid::{Ui, UiMain, UiState};

use druid::widget::Widget;
use druid::{Animation, AnimationCurve};
use druid::{BoxConstraints, Geometry, LayoutResult};
use druid::{HandlerCtx, Id, LayoutCtx, MouseEvent, PaintCtx};

/// A custom widget with animations.
struct AnimWidget(f64, u32);

impl Widget for AnimWidget {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        let fg = paint_ctx.render_ctx.solid_brush(self.1).unwrap();
        let (x, y) = geom.pos;
        let (x2, y2) = (x as f64 + geom.size.0 as f64 / 2., y as f64 + self.0 as f64);
        let circ = Circle::new((x2, y2), 50.);
        paint_ctx.render_ctx.fill(circ, &fg, FillRule::NonZero);
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

    fn animate(&mut self, anim: &Animation, ctx: &mut HandlerCtx) {
        self.0 = anim.current_value("a_float");
        self.1 = anim.current_value("a_color");
        ctx.request_anim_frame();
    }

    fn mouse(&mut self, event: &MouseEvent, ctx: &mut HandlerCtx) -> bool {
        if event.count > 0 {
            let anim = Animation::with_duration(2.0)
                .adding_component("a_float", AnimationCurve::OutElastic, 1.0, 350.0)
                .adding_component(
                    "a_color",
                    AnimationCurve::Linear,
                    0xFF_00_00_FF,
                    0x00_00_FF_FF,
                )
                //.looping(true)
                .reversing(true);
            ctx.animate(anim);
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
    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();
    let anim = AnimWidget(1.0, 0xCA_00_4a_FF).ui(&mut state);
    state.set_root(anim);
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Animation example");
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
