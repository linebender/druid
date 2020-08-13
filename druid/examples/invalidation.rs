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

//! Demonstrates how to debug invalidation regions, and also shows the
//! invalidation behavior of several build-in widgets.

use druid::kurbo::{Circle, Shape};
use druid::widget::prelude::*;
use druid::widget::{Button, Flex, Scroll, Split, TextBox};
use druid::{AppLauncher, Color, Data, Lens, LocalizedString, Point, WidgetExt, WindowDesc};

pub fn main() {
    let window = WindowDesc::new(build_widget).title(
        LocalizedString::new("invalidate-demo-window-title").with_placeholder("Invalidate demo"),
    );
    let state = AppState {
        label: "My label".into(),
        circle_pos: Point::new(0.0, 0.0),
    };
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(state)
        .expect("launch failed");
}

#[derive(Clone, Data, Lens)]
struct AppState {
    label: String,
    circle_pos: Point,
}

fn build_widget() -> impl Widget<AppState> {
    let mut col = Flex::column();
    col.add_child(TextBox::new().lens(AppState::label).padding(3.0));
    for i in 0..30 {
        col.add_child(Button::new(format!("Button {}", i)).padding(3.0));
    }
    Split::columns(Scroll::new(col), CircleView.lens(AppState::circle_pos)).debug_invalidation()
}

struct CircleView;

const RADIUS: f64 = 25.0;

impl Widget<Point> for CircleView {
    fn event(&mut self, ctx: &mut EventCtx, ev: &Event, data: &mut Point, _env: &Env) {
        if let Event::MouseDown(ev) = ev {
            // Move the circle to a new location, invalidating both the old and new locations.
            ctx.request_paint_rect(Circle::new(*data, RADIUS).bounding_box());
            ctx.request_paint_rect(Circle::new(ev.pos, RADIUS).bounding_box());
            *data = ev.pos;
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _ev: &LifeCycle, _data: &Point, _env: &Env) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _old: &Point, _new: &Point, _env: &Env) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &Point,
        _env: &Env,
    ) -> Size {
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Point, _env: &Env) {
        ctx.with_save(|ctx| {
            let rect = ctx.size().to_rect();
            ctx.clip(rect);
            ctx.fill(rect, &Color::WHITE);
            ctx.fill(Circle::new(*data, RADIUS), &Color::BLACK);
        })
    }
}
