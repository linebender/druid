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
//! This is useful when you want to have some kind of
//! generated content (like here), or some task that just
//! takes a long time but don't want to block the main thread
//! (waiting on an http request, some cpu intensive work etc.)

use instant::Instant;
use std::thread;
use std::time::Duration;

use druid::widget::prelude::*;
use druid::{AppLauncher, Color, Selector, Target, WidgetExt, WindowDesc};

// If you want to submit commands to an event sink you have to give it some kind
// of ID. The selector is that, it also assures the accompanying data-type is correct.
// look at the docs for `Selector` for more detail.
const SET_COLOR: Selector<Color> = Selector::new("event-example.set-color");

pub fn main() {
    let window = WindowDesc::new(make_ui).title("External Event Demo");

    let launcher = AppLauncher::with_window(window);

    // If we want to create commands from another thread `launcher.get_external_handle()`
    // should be used. For sending commands from within widgets you can always call
    // `ctx.submit_command`
    let event_sink = launcher.get_external_handle();
    // We create a new thread and generate colours in it.
    // This happens on a second thread so that we can run the UI in the
    // main thread. Generating some colours nicely follows the pattern for what
    // should be done like this: generating something over time
    // (like this or reacting to external events), or something that takes a
    // long time and shouldn't block main UI updates.
    thread::spawn(move || generate_colors(event_sink));

    launcher
        .use_simple_logger()
        .launch(Color::BLACK)
        .expect("launch failed");
}

fn generate_colors(event_sink: druid::ExtEventSink) {
    // This function is called in a separate thread, and runs until the program ends.
    // We take an `ExtEventSink` as an argument, we can use this event sink to send
    // commands to the main thread. Every time we generate a new colour we send it
    // to the main thread.
    let start_time = Instant::now();
    let mut color = Color::WHITE;

    loop {
        let time_since_start = (Instant::now() - start_time).as_nanos();
        let (r, g, b, _) = color.as_rgba8();

        // there is no logic here; it's a very silly way of mutating the color.
        color = match (time_since_start % 2, time_since_start % 3) {
            (0, _) => Color::rgb8(r.wrapping_add(3), g, b),
            (_, 0) => Color::rgb8(r, g.wrapping_add(3), b),
            (_, _) => Color::rgb8(r, g, b.wrapping_add(3)),
        };

        // We send a command to the event_sink. This command will be
        // send to the widgets, and widgets or controllers can look for this
        // event. We give it the data associated with the event and a target.
        // In this case this is just `Target::Auto`, look at the identity example
        // for more detail on how to send commands to specific widgets.
        if event_sink
            .submit_command(SET_COLOR, color.clone(), Target::Auto)
            .is_err()
        {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// A widget that displays a color.
struct ColorWell;

impl Widget<Color> for ColorWell {
    fn event(&mut self, _ctx: &mut EventCtx, event: &Event, data: &mut Color, _env: &Env) {
        match event {
            // This is where we handle our command.
            Event::Command(cmd) if cmd.is(SET_COLOR) => {
                // We don't do much data processing in the `event` method.
                // All we really do is just set the data. This causes a call
                // to `update` which requests a paint. You can also request a paint
                // during the event, but this should be reserved for changes to self.
                // For changes to `Data` always make `update` do the paint requesting.
                *data = cmd.get_unchecked(SET_COLOR).clone();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &Color, _: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &Color, data: &Color, _: &Env) {
        if old_data != data {
            ctx.request_paint()
        }
    }

    fn layout(&mut self, _: &mut LayoutCtx, bc: &BoxConstraints, _: &Color, _: &Env) -> Size {
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Color, _env: &Env) {
        let rect = ctx.size().to_rounded_rect(5.0);
        ctx.fill(rect, data);
    }
}

fn make_ui() -> impl Widget<Color> {
    ColorWell
        .fix_width(300.0)
        .fix_height(300.0)
        .padding(10.0)
        .center()
}
