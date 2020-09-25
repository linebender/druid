// Copyright 2020 The Druid Authors.
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

//! An example of sending commands from another thread.

use instant::Instant;
use std::thread;
use std::time::Duration;

use druid::kurbo::RoundedRect;
use druid::widget::prelude::*;
use druid::{
    AppLauncher, Color, Data, LocalizedString, Rect, Selector, Target, WidgetExt, WindowDesc,
};

const SET_COLOR: Selector<Color> = Selector::new("event-example.set-color");

/// A widget that displays a color.
struct ColorWell;

#[derive(Debug, Clone, Data)]
struct MyColor(#[data(same_fn = "color_eq")] Color);

fn color_eq(one: &Color, two: &Color) -> bool {
    one.as_rgba_u32() == two.as_rgba_u32()
}

fn split_rgba(rgba: &Color) -> (u8, u8, u8, u8) {
    let rgba = rgba.as_rgba_u32();
    (
        (rgba >> 24 & 255) as u8,
        ((rgba >> 16) & 255) as u8,
        ((rgba >> 8) & 255) as u8,
        (rgba & 255) as u8,
    )
}

impl ColorWell {
    pub fn new() -> Self {
        ColorWell
    }
}

impl Widget<MyColor> for ColorWell {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut MyColor, _env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(SET_COLOR) => {
                data.0 = cmd.get_unchecked(SET_COLOR).clone();
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &MyColor, _: &Env) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &MyColor, data: &MyColor, _: &Env) {
        if !old_data.same(data) {
            ctx.request_paint()
        }
    }

    fn layout(&mut self, _: &mut LayoutCtx, bc: &BoxConstraints, _: &MyColor, _: &Env) -> Size {
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &MyColor, _env: &Env) {
        let rect = Rect::ZERO.with_size(ctx.size());
        let rect = RoundedRect::from_rect(rect, 5.0);
        ctx.fill(rect, &data.0);
    }
}

pub fn main() {
    let window = WindowDesc::new(make_ui).title(
        LocalizedString::new("identity-demo-window-title").with_placeholder("External Event Demo"),
    );

    let launcher = AppLauncher::with_window(window);

    let event_sink = launcher.get_external_handle();
    let start_time = Instant::now();

    thread::spawn(move || {
        let mut last_color = Color::WHITE;

        loop {
            let time_since_start = (Instant::now() - start_time).as_nanos();
            let (r, g, b, _) = split_rgba(&last_color);

            // there is no logic here; it's a very silly way of mutating the color.
            let new_color = match (time_since_start % 2, time_since_start % 3) {
                (0, _) => Color::rgb8(r.wrapping_add(10), g, b),
                (_, 0) => Color::rgb8(r, g.wrapping_add(10), b),
                (_, _) => Color::rgb8(r, g, b.wrapping_add(10)),
            };

            last_color = new_color.clone();

            // if this fails we're shutting down
            if event_sink
                .submit_command(SET_COLOR, new_color, Target::Auto)
                .is_err()
            {
                break;
            }
            thread::sleep(Duration::from_millis(150));
        }
    });

    launcher
        .use_simple_logger()
        .launch(MyColor(Color::BLACK))
        .expect("launch failed");
}

fn make_ui() -> impl Widget<MyColor> {
    ColorWell::new()
        .fix_width(300.0)
        .fix_height(300.0)
        .padding(10.0)
        .center()
}
