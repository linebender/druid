// Copyright 2019 The Druid Authors.
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

//! An example of an animating widget.

use std::f64::consts::PI;

use druid::kurbo::{Circle, Line};
use druid::widget::prelude::*;
use druid::{AppLauncher, Color, LocalizedString, Point, Vec2, WindowDesc};

struct AnimWidget {
    t: f64,
}

impl Widget<u32> for AnimWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut u32, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                self.t = 0.0;
                ctx.request_anim_frame();
            }
            Event::AnimFrame(interval) => {
                ctx.request_paint();
                self.t += (*interval as f64) * 1e-9;
                if self.t < 1.0 {
                    ctx.request_anim_frame();
                }
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &u32, _env: &Env) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &u32, _data: &u32, _env: &Env) {}

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &u32,
        _env: &Env,
    ) -> Size {
        bc.constrain((100.0, 100.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &u32, _env: &Env) {
        let t = self.t;
        let center = Point::new(50.0, 50.0);
        ctx.paint_with_z_index(1, move |ctx| {
            let ambit = center + 45.0 * Vec2::from_angle((0.75 + t) * 2.0 * PI);
            ctx.stroke(Line::new(center, ambit), &Color::WHITE, 1.0);
        });

        ctx.fill(Circle::new(center, 50.0), &Color::BLACK);
    }
}

pub fn main() {
    let window = WindowDesc::new(|| AnimWidget { t: 0.0 }).title(
        LocalizedString::new("anim-demo-window-title")
            .with_placeholder("You spin me right round..."),
    );
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(0)
        .expect("launch failed");
}
