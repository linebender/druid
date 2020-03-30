// Copyright 2020 The xi-editor Authors.
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

use std::thread;
use std::time::{Duration, Instant};

use druid::kurbo::RoundedRect;
use druid::widget::prelude::*;
use druid::{AppLauncher, Color, Data, LocalizedString, Rect, Selector, WidgetExt, WindowDesc};

const SET_COLOR: Selector = Selector::new("event-example.set-color");

/// A widget that displays a color.
struct ColorWell;

#[derive(Debug, Clone, Data)]
struct MyColor(#[data(same_fn = "color_eq")] Color);

fn color_eq(one: &Color, two: &Color) -> bool {
    one.as_rgba_u32() == two.as_rgba_u32()
}

fn split_rgba(rgba: &Color) -> (f64, f64, f64, f64) {
    let rgba = rgba.as_rgba_u32();
    (
        (rgba >> 24) as f64 / 255.0,
        ((rgba >> 16) & 255) as f64 / 255.0,
        ((rgba >> 8) & 255) as f64 / 255.0,
        (rgba & 255) as f64 / 255.0,
    )
}

fn color_average(one: &Color, two: &Color) -> Color {
    let one = split_rgba(one);
    let two = split_rgba(two);
    Color::rgb8(
        ((one.0 + two.0 * 19.0) / 20.0 * 255.0) as u8,
        ((one.1 + two.1 * 19.0) / 20.0 * 255.0) as u8,
        ((one.2 + two.2 * 19.0) / 20.0 * 255.0) as u8,
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
            Event::Command(cmd) if cmd.selector == SET_COLOR => {
                data.0 = cmd.get_object::<Color>().unwrap().clone();
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

fn main() {
    let window = WindowDesc::new(make_ui).title(
        LocalizedString::new("identity-demo-window-title").with_placeholder("External Event Demo"),
    );

    let launcher = AppLauncher::with_window(window);

    let event_sink = launcher.get_external_handle();
    let start_time = Instant::now();

    thread::spawn(move || {
        let mut last_color = Color::WHITE;

        loop {
            let time_since_start = Instant::now() - start_time;
            let bits = (time_since_start.as_nanos() % (0xFFFFFF)) as u32;

            // there is no logic here; it's a very silly way of creating a color.
            let mask = 0x924924;
            let red = bits & mask;
            let red = (red >> 16 | red >> 8 | red) & 0xFF;
            let green = bits & mask >> 1;
            let green = (green >> 16 | green >> 8 | green) & 0xFF;
            let blue = bits & mask >> 2;
            let blue = (blue >> 16 | blue >> 8 | blue) & 0xFF;

            let new_color = Color::rgb8(red as u8, green as u8, blue as u8);
            let next_color = color_average(&new_color, &last_color);
            last_color = next_color.clone();

            // if this fails we're shutting down
            if let Err(_) = event_sink.submit_command(SET_COLOR, next_color, None) {
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
