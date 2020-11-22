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

//! An example of sending commands to specific widgets.
//!
//! This example is fairly contrived; the basic idea is that there is one
//! "top widget", displaying random colors, and are three rows of widgets,
//! each containing a ColorWell and two buttons.
//! One button 'pins' the ColorWell, sending it a color to display.
//! The second button 'syncs' the ColorWell, which makes it start displaying
//! the same random colors as the top widget.
//!
//! The key insight is that each button is linked to a specific ColorWell, and
//! can send messages that are only handled by that widget.
//!
//! This is a contrived example; if you were designing a real app you might
//! choose a different mechanism (such as just representing all of this state
//! in your `Data` type) but this is an example, and I couldn't think of anything
//! better. ¯\_(ツ)_/¯
//!
//! This example is based on the async_function example. It might be usefull to
//! look at that example if you are not sure how the colours are generated.

use instant::Instant;
use std::thread;
use std::time::Duration;

use druid::kurbo::RoundedRect;
use druid::widget::prelude::*;
use druid::widget::{Button, Controller, CrossAxisAlignment, Flex, WidgetId};
use druid::{AppLauncher, Color, Rect, Selector, Target, WidgetExt, WindowDesc};

const CYCLE_DURATION: Duration = Duration::from_millis(100);

const PIN_COLOR: Selector = Selector::new("identity-example.pin-color");
const SYNC_COLOR: Selector = Selector::new("identity-example.sync-color");
const SET_COLOR: Selector<Color> = Selector::new("event-example.set-color");

pub fn main() {
    let window = WindowDesc::new(make_ui).title("Color Freezing Fun");
    let launcher = AppLauncher::with_window(window);

    let event_sink = launcher.get_external_handle();
    thread::spawn(move || generate_colors(event_sink));

    launcher
        .use_simple_logger()
        .launch(Color::BLACK)
        .expect("launch failed");
}

/// A constant `WidgetId`. This may be passed around and can be reused when
/// rebuilding a widget graph; however it should only ever be associated with
/// a single widget at a time.
const ID_ONE: WidgetId = WidgetId::reserved(1);

fn make_ui() -> impl Widget<Color> {
    // We can also generate these dynamically whenever we need it.
    let id_two = WidgetId::next();
    let id_three = WidgetId::next();

    let mut column = Flex::column().with_flex_child(ColorWell::new(true), 1.0);
    // This doesn't need to be a loop, but it allows us to separate the creation of the buttons and the colorwell.
    for &id in &[ID_ONE, id_two, id_three] {
        // Here we can see the `id` to make sure all the buttons correlate with the colorwell.
        // We give the colorwell an id, and we use that same id to target our commands to that widget specifically.
        // This allows us to send commands to only one widget, and not the whole window for example.
        // In this case, when the buttons are clicked we send a command to the corresponding colorwell.
        let colorwell = ColorWell::new(false).with_id(id);
        let pin_button = Button::<Color>::new("pin")
            .on_click(move |ctx, _data, _env| ctx.submit_command(PIN_COLOR.to(id)));
        let sync_button = Button::<Color>::new("sync")
            .on_click(move |ctx, _data, _env| ctx.submit_command(SYNC_COLOR.to(id)));

        column = column.with_default_spacer().with_flex_child(
            Flex::row()
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .with_flex_child(colorwell, 1.0)
                .with_default_spacer()
                .with_child(pin_button)
                .with_default_spacer()
                .with_child(sync_button),
            0.5,
        );
    }
    column.padding(10.0).controller(ColorWellController)
}

// This controler manages the synced color. It is responsable for changing the
// `Data` when a new colour is send in via the `SET_COLOR` event.
struct ColorWellController;
impl<W: Widget<Color>> Controller<Color, W> for ColorWellController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut Color,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(SET_COLOR) => {
                *data = cmd.get_unchecked(SET_COLOR).clone();
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}

/// A widget that displays a color.
///
/// For the purpose of this fairly contrived demo, this widget works in one of two ways:
/// either it is the main big color widget, which randomly cycles through colors, or else
/// it is one of the freezable widgets, which can synchronise witht the main one, or pin itself.
/// The implementation of this is not really relevant, the example is about the ids not this
/// widget.
struct ColorWell(Option<Color>);
impl ColorWell {
    pub fn new(randomize: bool) -> Self {
        let color = if randomize {
            None
        } else {
            Some(Color::rgba(0., 0., 0., 0.2))
        };
        ColorWell(color)
    }
}

impl Widget<Color> for ColorWell {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Color, _env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(PIN_COLOR) => {
                self.0 = Some(data.clone());
                ctx.request_paint();
            }
            Event::Command(cmd) if cmd.is(SYNC_COLOR) => self.0 = None,
            _ => (),
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &Color, _: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &Color, data: &Color, _: &Env) {
        if old_data != data && self.0.is_none() {
            ctx.request_paint()
        }
    }

    fn layout(&mut self, _: &mut LayoutCtx, bc: &BoxConstraints, _: &Color, _: &Env) -> Size {
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Color, _env: &Env) {
        let rect = Rect::ZERO.with_size(ctx.size());
        let rect = RoundedRect::from_rect(rect, 5.0);
        let color = self.0.as_ref().unwrap_or(data);
        ctx.fill(rect, color);
    }
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
        thread::sleep(CYCLE_DURATION);
    }
}
