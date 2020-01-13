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

//! An example of sending commands to specific widgets.
//!
//! This example is fairly contrived; the basic idea is that there are three
//! rows of widgets, each containing a ColorWell and two buttons. One button
//! 'freezes' the ColorWell, sending it a color to display. The second button
//! 'unfreezes' the ColorWell, which makes it start displaying random colors.
//!
//! The key insight is that each button is linked to a specific ColorWell, and
//! can send messages that are only handled by that widget.
//!
//! This is a contrived example; if you were designing a real app you might
//! choose a different mechanism (such as just representing all of this state
//! in your `Data` type) but this is an example, and I couldn't think of anything
//! better. ¯\_(ツ)_/¯

use std::time::{Duration, Instant};

use druid::kurbo::RoundedRect;
use druid::widget::{Button, Flex, IdentityWrapper, WidgetExt};
use druid::{
    AppLauncher, BoxConstraints, Color, Command, Data, Env, Event, EventCtx, LayoutCtx, Lens,
    LifeCycle, LifeCycleCtx, LocalizedString, PaintCtx, Rect, RenderContext, Selector, Size,
    TimerToken, UpdateCtx, Widget, WindowDesc,
};

const CYCLE_DURATION: Duration = Duration::from_millis(200);

const FREEZE_COLOR: Selector = Selector::new("identity-example.freeze-color");
const UNFREEZE_COLOR: Selector = Selector::new("identity-example.unfreeze-color");
const SET_INITIAL_TOKEN: Selector = Selector::new("identity-example.set-initial-token");

/// Honestly: it's just a color in fancy clothing.
#[derive(Debug, Clone, Data, Lens)]
struct OurData {
    #[druid(same_fn = "color_eq")]
    color: Color,
}

fn color_eq(one: &Color, two: &Color) -> bool {
    one.as_rgba_u32() == two.as_rgba_u32()
}

/// A widget that displays a color.
///
/// For the purpose of this fairly contrived demo, this widget works in one of two ways:
/// either it is the main big color widget, which randomly cycles through colors, or else
/// it is one of the freezable widgets, which can receive a command with a color to display.
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
            Some(Color::rgba(0., 0., 0., 0.))
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
                let time_since_start = Instant::now() - self.start;
                // there is no logic here; it's a very silly way of creating a color.
                let bits = (time_since_start.as_nanos() % (0xFFFFFF)) as u32;
                let mask = 0x924924;
                let red = bits & mask;
                let red = (red >> 16 | red >> 8 | red) & 0xFF;
                let green = bits & mask >> 1;
                let green = (green >> 16 | green >> 8 | green) & 0xFF;
                let blue = bits & mask >> 2;
                let blue = (blue >> 16 | blue >> 8 | blue) & 0xFF;

                data.color = Color::rgb8(red as u8, green as u8, blue as u8);
                self.token = ctx.request_timer(Instant::now() + CYCLE_DURATION);
                ctx.invalidate();
            }

            Event::Command(cmd) if cmd.selector == SET_INITIAL_TOKEN => {
                self.token = ctx.request_timer(Instant::now() + CYCLE_DURATION);
            }

            Event::Command(cmd) if cmd.selector == FREEZE_COLOR => {
                self.frozen = cmd
                    .get_object::<Color>()
                    .cloned()
                    .expect("payload is always a Color")
                    .into();
            }
            Event::Command(cmd) if cmd.selector == UNFREEZE_COLOR => self.frozen = None,
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &OurData, _: &Env) {
        match event {
            LifeCycle::WindowConnected if self.randomize => {
                ctx.submit_command(SET_INITIAL_TOKEN, ctx.widget_id());
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&OurData>, data: &OurData, _: &Env) {
        if match old_data {
            Some(d) => !d.same(data),
            None => true,
        } {
            ctx.invalidate()
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

fn main() {
    let window = WindowDesc::new(make_ui).title(
        LocalizedString::new("identity-demo-window-title")
            .with_placeholder("Color Freezing Fun".into()),
    );
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(OurData {
            color: Color::BLACK,
        })
        .expect("launch failed");
}

fn make_ui() -> impl Widget<OurData> {
    let (id_one, one) = IdentityWrapper::wrap(ColorWell::new(false).padding(10.));
    let (id_two, two) = IdentityWrapper::wrap(ColorWell::new(false).padding(10.));
    let (id_three, three) = IdentityWrapper::wrap(ColorWell::new(false).padding(10.));
    Flex::column()
        .with_child(ColorWell::new(true).padding(10.0), 1.0)
        .with_child(
            Flex::row()
                .with_child(one, 1.0)
                .with_child(
                    Button::<OurData>::new("freeze", move |ctx, data, _env| {
                        ctx.submit_command(Command::new(FREEZE_COLOR, data.color.clone()), id_one)
                    })
                    .padding(10.0),
                    0.5,
                )
                .with_child(
                    Button::<OurData>::new("unfreeze", move |ctx, _, _env| {
                        ctx.submit_command(UNFREEZE_COLOR, id_one)
                    })
                    .padding(10.0),
                    0.5,
                ),
            0.5,
        )
        .with_child(
            Flex::row()
                .with_child(two, 1.)
                .with_child(
                    Button::<OurData>::new("freeze", move |ctx, data, _env| {
                        ctx.submit_command(Command::new(FREEZE_COLOR, data.color.clone()), id_two)
                    })
                    .padding(10.0),
                    0.5,
                )
                .with_child(
                    Button::<OurData>::new("unfreeze", move |ctx, _, _env| {
                        ctx.submit_command(UNFREEZE_COLOR, id_two)
                    })
                    .padding(10.0),
                    0.5,
                ),
            0.5,
        )
        .with_child(
            Flex::row()
                .with_child(three, 1.)
                .with_child(
                    Button::<OurData>::new("freeze", move |ctx, data, _env| {
                        ctx.submit_command(Command::new(FREEZE_COLOR, data.color.clone()), id_three)
                    })
                    .padding(10.0),
                    0.5,
                )
                .with_child(
                    Button::<OurData>::new("unfreeze", move |ctx, _, _env| {
                        ctx.submit_command(UNFREEZE_COLOR, id_three)
                    })
                    .padding(10.0),
                    0.5,
                ),
            0.5,
        )
}
