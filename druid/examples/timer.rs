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

//! An example of a timer.

use std::time::Duration;

use druid::widget::prelude::*;
use druid::widget::BackgroundBrush;
use druid::{AppLauncher, Color, LocalizedString, Point, Rect, TimerToken, WidgetPod, WindowDesc};

static TIMER_INTERVAL: Duration = Duration::from_millis(10);

struct TimerWidget {
    timer_id: TimerToken,
    simple_box: WidgetPod<u32, SimpleBox>,
    pos: Point,
}

impl TimerWidget {
    /// Move the box towards the right, until it reaches the edge,
    /// then reset it to the left but move it to another row.
    fn adjust_box_pos(&mut self, container_size: Size) {
        let box_size = self.simple_box.layout_rect().size();
        self.pos.x += 2.;
        if self.pos.x + box_size.width > container_size.width {
            self.pos.x = 0.;
            self.pos.y += box_size.height;
            if self.pos.y + box_size.height > container_size.height {
                self.pos.y = 0.;
            }
        }
    }
}

impl Widget<u32> for TimerWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut u32, env: &Env) {
        match event {
            Event::WindowConnected => {
                // Start the timer when the application launches
                self.timer_id = ctx.request_timer(TIMER_INTERVAL);
            }
            Event::Timer(id) => {
                if *id == self.timer_id {
                    self.adjust_box_pos(ctx.size());
                    ctx.request_layout();
                    self.timer_id = ctx.request_timer(TIMER_INTERVAL);
                }
            }
            _ => (),
        }
        self.simple_box.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &u32, env: &Env) {
        self.simple_box.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &u32, data: &u32, env: &Env) {
        self.simple_box.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &u32, env: &Env) -> Size {
        let size = self.simple_box.layout(ctx, &bc.loosen(), data, env);
        let rect = Rect::from_origin_size(self.pos, size);
        self.simple_box.set_layout_rect(ctx, data, env, rect);
        bc.constrain((500.0, 500.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &u32, env: &Env) {
        self.simple_box.paint(ctx, data, env);
    }
}

struct SimpleBox;

impl Widget<u32> for SimpleBox {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut u32, _env: &Env) {}

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &u32, _env: &Env) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &u32, _data: &u32, _env: &Env) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &u32,
        _env: &Env,
    ) -> Size {
        bc.constrain((50.0, 50.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &u32, env: &Env) {
        let mut background = if ctx.is_hot() {
            BackgroundBrush::Color(Color::rgb8(200, 55, 55))
        } else {
            BackgroundBrush::Color(Color::rgb8(30, 210, 170))
        };
        background.paint(ctx, data, env);
    }
}

pub fn main() {
    let window = WindowDesc::new(|| TimerWidget {
        timer_id: TimerToken::INVALID,
        simple_box: WidgetPod::new(SimpleBox),
        pos: Point::ZERO,
    })
    .with_min_size((200., 200.))
    .title(LocalizedString::new("timer-demo-window-title").with_placeholder("Look at it go!"));

    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(0u32)
        .expect("launch failed");
}
