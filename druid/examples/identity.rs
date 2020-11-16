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

use instant::Instant;
use std::time::Duration;

use druid::kurbo::RoundedRect;
use druid::widget::prelude::*;
use druid::widget::{Button, CrossAxisAlignment, Flex, WidgetId};
use druid::{
    AppLauncher, Color, Data, LocalizedString, Rect, Selector, TimerToken, WidgetExt, WindowDesc,
};

const CYCLE_DURATION: Duration = Duration::from_millis(100);

const PIN_COLOR: Selector<Color> = Selector::new("identity-example.pin-color");
const SYNC_COLOR: Selector = Selector::new("identity-example.sync-color");

/// Honestly: it's just a color in fancy clothing.
#[derive(Clone, Data)]
struct OurData {
    color: Color,
}

pub fn main() {
    let window = WindowDesc::new(make_ui).title(
        LocalizedString::new("identity-demo-window-title").with_placeholder("Color Freezing Fun"),
    );
    let data = OurData {
        color: Color::BLACK,
    };
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

/// A constant `WidgetId`. This may be passed around and can be reused when
/// rebuilding a widget graph; however it should only ever be associated with
/// a single widget at a time.
const ID_ONE: WidgetId = WidgetId::reserved(1);

fn make_ui() -> impl Widget<OurData> {
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
        let pin_button = Button::<OurData>::new("pin").on_click(move |ctx, data, _env| {
            ctx.submit_command(PIN_COLOR.with(data.color.clone()).to(id))
        });
        let sync_button = Button::<OurData>::new("sync")
            .on_click(move |ctx, _, _env| ctx.submit_command(SYNC_COLOR.to(id)));

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
    column.padding(10.0)
}

/// A widget that displays a color.
///
/// For the purpose of this fairly contrived demo, this widget works in one of two ways:
/// either it is the main big color widget, which randomly cycles through colors, or else
/// it is one of the freezable widgets, which can receive a command with a color to display.
/// The implementation of this is not really relevant, the example is about the ids not this
/// widget.
struct ColorWell {
    randomize: bool,
    token: TimerToken,
    start: Instant,
    frozen: Option<Color>,
}

impl ColorWell {
    pub fn new(randomize: bool) -> Self {
        let frozen = if randomize {
            None
        } else {
            Some(Color::rgba(0., 0., 0., 0.2))
        };
        ColorWell {
            randomize,
            token: TimerToken::INVALID,
            start: Instant::now(),
            frozen,
        }
    }
}

impl Widget<OurData> for ColorWell {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut OurData, _env: &Env) {
        match event {
            Event::Timer(t) if t == &self.token => {
                let time_since_start = (Instant::now() - self.start).as_nanos();
                let (r, g, b, _) = data.color.as_rgba8();

                // there is no logic here; it's a very silly way of mutating the color.
                data.color = match (time_since_start % 2, time_since_start % 3) {
                    (0, _) => Color::rgb8(r.wrapping_add(10), g, b),
                    (_, 0) => Color::rgb8(r, g.wrapping_add(10), b),
                    (_, _) => Color::rgb8(r, g, b.wrapping_add(10)),
                };

                self.token = ctx.request_timer(CYCLE_DURATION);
                ctx.request_paint();
            }
            Event::WindowConnected if self.randomize => {
                self.token = ctx.request_timer(CYCLE_DURATION);
            }
            Event::Command(cmd) if cmd.is(PIN_COLOR) => {
                self.frozen = cmd.get(PIN_COLOR).cloned();
            }
            Event::Command(cmd) if cmd.is(SYNC_COLOR) => self.frozen = None,
            _ => (),
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &OurData, _: &Env) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &OurData, data: &OurData, _: &Env) {
        if old_data.color != data.color {
            ctx.request_paint()
        }
    }

    fn layout(&mut self, _: &mut LayoutCtx, bc: &BoxConstraints, _: &OurData, _: &Env) -> Size {
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &OurData, _env: &Env) {
        let rect = Rect::ZERO.with_size(ctx.size());
        let rect = RoundedRect::from_rect(rect, 5.0);
        let color = self.frozen.as_ref().unwrap_or(&data.color);
        ctx.fill(rect, color);
    }
}
