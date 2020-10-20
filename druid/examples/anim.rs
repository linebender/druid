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

//! An example of an animating widget. It is just a widget that
//! requests an animation frame when it needs to, and draws the frame in the
//! `paint` method.
//! Once the animation is over it simply stops requesting animation frames.
//! In this case the animation time(t) is in the data so that other widgets
//! can also read the state of the animation if needed. This can also be in
//! the widget itself, so that it is not part of the data.

use std::f64::consts::PI;

use druid::kurbo::{Circle, Line};
use druid::widget::prelude::*;
use druid::{AppLauncher, Color, LocalizedString, Point, Vec2, WindowDesc};

struct AnimWidget {}

impl Widget<f64> for AnimWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut f64, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                *data = 0.0;
                ctx.request_anim_frame();
            }
            Event::AnimFrame(interval) => {
                *data += (*interval as f64) * 1e-9;
                if *data < 1.0 {
                    ctx.request_anim_frame();
                } else {
                    *data = 0.0;
                }
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &f64, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &f64, data: &f64, _env: &Env) {
        if data != old_data {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &f64,
        _env: &Env,
    ) -> Size {
        bc.constrain((100.0, 100.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &f64, _env: &Env) {
        let t = *data;
        let center = Point::new(50.0, 50.0);
        ctx.paint_with_z_index(1, move |ctx| {
            let ambit = center + 45.0 * Vec2::from_angle((0.75 + t) * 2.0 * PI);
            ctx.stroke(Line::new(center, ambit), &Color::WHITE, 1.0);
        });

        ctx.fill(Circle::new(center, 50.0), &Color::BLACK);
    }
}

pub fn main() {
    let window = WindowDesc::new(|| AnimWidget {}).title(
        LocalizedString::new("anim-demo-window-title")
            .with_placeholder("You spin me right round..."),
    );
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(0.0)
        .expect("launch failed");
}
