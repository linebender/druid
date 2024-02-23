// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! An example of sending commands from another thread.
//! This is useful when you want to have some kind of
//! generated content (like here), or some task that just
//! takes a long time but don't want to block the main thread
//! (waiting on an http request, some cpu intensive work etc.)

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use instant::Instant;
use std::thread;
use std::time::Duration;

use druid::widget::Painter;
use druid::{AppLauncher, Color, RenderContext, Widget, WidgetExt, WindowDesc};

pub fn main() {
    let window = WindowDesc::new(make_ui()).title("External Event Demo");

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
        .log_to_console()
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

        // schedule idle callback to change the data
        event_sink.add_idle_callback(move |data: &mut Color| {
            *data = color;
        });
        thread::sleep(Duration::from_millis(20));
    }
}

fn make_ui() -> impl Widget<Color> {
    Painter::new(|ctx, data, _env| {
        let rect = ctx.size().to_rounded_rect(5.0);
        ctx.fill(rect, data);
    })
    .fix_width(300.0)
    .fix_height(300.0)
    .padding(10.0)
    .center()
}
