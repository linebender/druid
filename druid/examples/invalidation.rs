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

use druid::im::Vector;
use druid::kurbo::{self, Shape};
use druid::widget::prelude::*;
use druid::widget::{Button, Flex, Scroll, Split, TextBox};
use druid::{AppLauncher, Color, Data, Lens, LocalizedString, Point, WidgetExt, WindowDesc};

use instant::Instant;

pub fn main() {
    let window = WindowDesc::new(build_widget).title(
        LocalizedString::new("invalidate-demo-window-title").with_placeholder("Invalidate demo"),
    );
    let state = AppState {
        label: "My label".into(),
        circles: Vector::new(),
    };
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(state)
        .expect("launch failed");
}

#[derive(Clone, Data, Lens)]
struct AppState {
    label: String,
    circles: Vector<Circle>,
}

fn build_widget() -> impl Widget<AppState> {
    let mut col = Flex::column();
    col.add_child(TextBox::new().lens(AppState::label).padding(3.0));
    for i in 0..30 {
        col.add_child(Button::new(format!("Button {}", i)).padding(3.0));
    }
    Split::columns(Scroll::new(col), CircleView.lens(AppState::circles)).debug_invalidation()
}

struct CircleView;

#[derive(Clone, Data)]
struct Circle {
    pos: Point,
    #[data(same_fn = "PartialEq::eq")]
    time: Instant,
}

const RADIUS: f64 = 25.0;

impl Widget<Vector<Circle>> for CircleView {
    fn event(&mut self, ctx: &mut EventCtx, ev: &Event, data: &mut Vector<Circle>, _env: &Env) {
        if let Event::MouseDown(ev) = ev {
            if ev.mods.shift() {
                data.push_back(Circle {
                    pos: ev.pos,
                    time: Instant::now(),
                });
            } else if ev.mods.ctrl() {
                data.retain(|c| {
                    if (c.pos - ev.pos).hypot() > RADIUS {
                        true
                    } else {
                        ctx.request_paint_rect(kurbo::Circle::new(c.pos, RADIUS).bounding_box());
                        false
                    }
                });
            } else {
                // Move the circle to a new location, invalidating the old locations. The new location
                // will be invalidated during AnimFrame.
                for c in data.iter() {
                    ctx.request_paint_rect(kurbo::Circle::new(c.pos, RADIUS).bounding_box());
                }
                data.clear();
                data.push_back(Circle {
                    pos: ev.pos,
                    time: Instant::now(),
                });
            }
            ctx.request_anim_frame();
        } else if let Event::AnimFrame(_) = ev {
            for c in &*data {
                ctx.request_paint_rect(kurbo::Circle::new(c.pos, RADIUS).bounding_box());
            }
            if !data.is_empty() {
                ctx.request_anim_frame();
            }
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _ev: &LifeCycle,
        _data: &Vector<Circle>,
        _env: &Env,
    ) {
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx,
        _old: &Vector<Circle>,
        _new: &Vector<Circle>,
        _env: &Env,
    ) {
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &Vector<Circle>,
        _env: &Env,
    ) -> Size {
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Vector<Circle>, _env: &Env) {
        ctx.with_save(|ctx| {
            let rect = ctx.size().to_rect();
            ctx.clip(rect);
            ctx.fill(rect, &Color::WHITE);
            let now = Instant::now();
            for c in data {
                let color =
                    Color::BLACK.with_alpha(now.duration_since(c.time).as_secs_f64().cos().abs());
                ctx.fill(kurbo::Circle::new(c.pos, RADIUS), &color);
            }
        });
    }
}
