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
use druid::shell::{runloop, WindowBuilder};
use druid::{
    Action, BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, TimerToken,
    UpdateCtx, Widget,
};
use druid::{UiMain, UiState};

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
            let brush = paint_ctx.solid_brush(Color::WHITE);
            paint_ctx.stroke(Line::new((10.0, 10.0), (10.0, 50.0)), &brush, 1.0, None);
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

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        _data: &mut u32,
        _env: &Env,
    ) -> Option<Action> {
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
        None
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&u32>, _data: &u32, _env: &Env) {}
}

fn main() {
    druid::shell::init();

    let mut run_loop = runloop::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let root = TimerWidget {
        timer_id: TimerToken::INVALID,
        on: false,
    };
    let state = UiState::new(root, 0u32);
    builder.set_title("Timer example");
    builder.set_handler(Box::new(UiMain::new(state)));
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
