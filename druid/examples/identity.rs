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
//! This example is fairly contrived; the basic idea is that there are two counters,
//! and two buttons. If you press button one, counter one goes up. If you press button
//! two counter two goes up.
//!
//! The key insight is that the data is not mutated by the button directly;
//! instead the button sends a command to a separate controller widget
//! that performs the actual mutation.
//!
//! If you were designing a real app you might choose a different mechanism
//! (such as just just changing the `Data` in the on_click); however there are
//! other circumstances where widgets may need to communicate with specific
//! other widgets, and identity is a useful mechanism in those cases.

use druid::widget::prelude::*;
use druid::widget::{Button, Controller, Flex, Label, WidgetId};
use druid::{AppLauncher, Data, Lens, Selector, WidgetExt, WindowDesc};

const INCREMENT: Selector = Selector::new("identity-example.increment");

#[derive(Clone, Data, Lens)]
struct OurData {
    counter_one: u64,
    counter_two: u64,
}

pub fn main() {
    let window = WindowDesc::new(make_ui()).title("identity example");
    let data = OurData {
        counter_one: 0,
        counter_two: 0,
    };
    AppLauncher::with_window(window)
        .log_to_console()
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
    // We have a column with 2 labels and 2 buttons.
    // Each of the 2 labels only have acces to their own counter and are given a `WidgetId`.
    // Both labels have a controler, this handles commands send to children.
    // The 2 buttons send a command when clicked. Both send the exact same command.
    // The key diference is that they both give a diferent `WidgetId` as target.
    // This means that only the corresponding controler gets the command, and increments their counter.
    Flex::column()
        .with_child(
            Label::dynamic(|data, _| format!("One: {}", data))
                .controller(LabelControler)
                .with_id(ID_ONE)
                .lens(OurData::counter_one)
                .padding(2.0),
        )
        .with_child(
            Label::dynamic(|data, _| format!("Two: {}", data))
                .controller(LabelControler)
                .with_id(id_two)
                .lens(OurData::counter_two)
                .padding(2.0),
        )
        .with_child(
            Button::<OurData>::new("Increment one")
                .on_click(|ctx, _data, _env| ctx.submit_command(INCREMENT.to(ID_ONE)))
                .padding(2.0),
        )
        .with_child(
            Button::<OurData>::new("Increment two")
                .on_click(move |ctx, _data, _env| ctx.submit_command(INCREMENT.to(id_two)))
                .padding(2.0),
        )
        .padding(10.0)
}

struct LabelControler;

impl Controller<u64, Label<u64>> for LabelControler {
    fn event(
        &mut self,
        child: &mut Label<u64>,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut u64,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(INCREMENT) => *data += 1,
            _ => child.event(ctx, event, data, env),
        }
    }
}
