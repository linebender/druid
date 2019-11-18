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

//! An example of a timer.

use std::time::{Duration, Instant};

use druid::kurbo::{Line, Size};
use druid::piet::{Color, RenderContext};
use druid::{
    AppLauncher, BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, TimerToken,
    UpdateCtx, Widget, WindowDesc,
};

struct TimerWidget {
    timer_id: TimerToken,
    on: bool,
}

impl Widget<u32> for TimerWidget {
    fn paint(
        &mut self,
        paint_ctx: &mut PaintCtx,
        _base_state: &BaseState,
        _data: &u32,
        _env: &Env,
    ) {
        if self.on {
            paint_ctx.stroke(Line::new((10.0, 10.0), (10.0, 50.0)), &Color::WHITE, 1.0);
        }
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &u32,
        _env: &Env,
    ) -> Size {
        bc.constrain((100.0, 100.0))
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut u32, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                self.on = !self.on;
                ctx.invalidate();
                let deadline = Instant::now() + Duration::from_millis(500);
                self.timer_id = ctx.request_timer(deadline);
            }
            Event::Timer(id) => {
                if *id == self.timer_id {
                    self.on = !self.on;
                    ctx.invalidate();
                    let deadline = Instant::now() + Duration::from_millis(500);
                    self.timer_id = ctx.request_timer(deadline);
                }
            }
            _ => (),
        }
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&u32>, _data: &u32, _env: &Env) {}
}

fn main() {
    let window = WindowDesc::new(|| TimerWidget {
        timer_id: TimerToken::INVALID,
        on: false,
    });

    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(0u32)
        .expect("launch failed");
}
