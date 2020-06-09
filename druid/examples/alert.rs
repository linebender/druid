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

//! Alert dialogs in action.

use std::collections::HashMap;
use std::time::Duration;

use druid::widget::prelude::*;
use druid::widget::{Button, Controller, Flex, Label, MainAxisAlignment, Padding};
use druid::{AlertButton, AlertOptions, AlertToken, AppLauncher, WidgetExt, WindowDesc};
use druid::{Data, TimerToken};

const ALERT_BUTTON_TO_LEFT: AlertButton = AlertButton::new("Increase left");

#[derive(Debug, Clone, Data)]
struct State {
    left: usize,
    right: usize,
    bits: usize,
}

fn main() {
    let main_window = WindowDesc::new(ui_builder).title("Alerts everywhere");
    let state = State {
        left: 5,
        right: 5,
        bits: 0,
    };
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(state)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<State> {
    let button_new_window = Button::new("New window").on_click(|ctx, _data, _env| {
        let new_window = WindowDesc::new(ui_builder);
        ctx.new_window(new_window);
    });
    let button_kitchen = Button::new("Kitchen sink").on_click(|ctx, _data, _env| {
        let opts = AlertOptions::error()
            .context("Heavyweight example")
            .message("Don't do this at home!")
            .description(
                "This is an example of how the alert system works, \
                but not an example of a good user experience.",
            )
            .primary(AlertButton::new("Get me out of here"))
            .alternative(AlertButton::new("Refrigerate lemons"))
            .alternative(AlertButton::new("Dip fat-free cheese in fat"))
            .cancel(AlertButton::new("Custom cancel"));
        ctx.alert(opts);
    });
    let button_flood = Button::new("Three in a row").on_click(|ctx, _data, _env| {
        for i in 0..3 {
            let opts = AlertOptions::warning().message(format!("Alert #{}", i + 1));
            ctx.alert(opts);
        }
    });
    let button_manage = Button::<State>::new("Manage score")
        .on_click(|ctx, _data, _env| {
            let opts = AlertOptions::information()
                .context("Manage score")
                .message("How would you like to manage the score?")
                .primary(ALERT_BUTTON_TO_LEFT)
                .alternative(AlertButton::dynamic("Increase right"))
                .cancelable();
            ctx.alert(opts);
        })
        .controller(ManageButtonController);
    let button_bits = Button::<State>::dynamic(|data, _| format!("Bits: {:05b}", data.bits))
        .controller(BitsButtonController::new());

    let label_left = Label::new(|data: &State, _: &_| format!("{}", data.left));
    let label_right = Label::new(|data: &State, _: &_| format!("{}", data.right));

    let label_row = Flex::row()
        .must_fill_main_axis(true)
        .main_axis_alignment(MainAxisAlignment::SpaceBetween)
        .with_child(label_left)
        .with_child(label_right);

    Flex::column()
        .main_axis_alignment(MainAxisAlignment::End)
        .with_flex_child(Padding::new(5.0, button_new_window), 1.0)
        .with_flex_child(Padding::new(5.0, button_kitchen), 1.0)
        .with_flex_child(Padding::new(5.0, button_manage), 1.0)
        .with_flex_child(Padding::new(5.0, button_flood), 1.0)
        .with_flex_child(Padding::new(5.0, button_bits), 1.0)
        .with_flex_child(Padding::new(20.0, label_row), 1.0)
}

struct ManageButtonController;

impl<W: Widget<State>> Controller<State, W> for ManageButtonController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut State,
        env: &Env,
    ) {
        match event {
            Event::AlertResponse(response) => {
                if response.button() == Some(&ALERT_BUTTON_TO_LEFT) {
                    if data.right > 0 {
                        data.left += 1;
                        data.right -= 1;
                    }
                } else if response.button() == Some(&AlertButton::dynamic("Increase right")) {
                    if data.left > 0 {
                        data.left -= 1;
                        data.right += 1;
                    }
                }
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}

const BITS_BUTTON_SET: AlertButton = AlertButton::new("Set");
const BITS_BUTTON_CLEAR: AlertButton = AlertButton::new("Clear");

struct BitsButtonController {
    counter: usize,
    timer_token: TimerToken,
    tokens: HashMap<AlertToken, usize>,
}

impl BitsButtonController {
    pub fn new() -> BitsButtonController {
        BitsButtonController {
            counter: 0,
            timer_token: TimerToken::INVALID,
            tokens: HashMap::new(),
        }
    }
}

impl<W: Widget<State>> Controller<State, W> for BitsButtonController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut State,
        env: &Env,
    ) {
        match event {
            Event::MouseUp(_) => {
                if ctx.is_active() && ctx.is_hot() {
                    self.timer_token = ctx.request_timer(Duration::from_millis(1));
                }
            }
            Event::Timer(timer_token) => {
                if self.timer_token == *timer_token {
                    if self.counter == 0 {
                        self.tokens.clear();
                    }
                    self.counter += 1;
                    let opts = AlertOptions::new()
                        .message(format!("What about bit #{}?", self.counter))
                        .primary(BITS_BUTTON_SET)
                        .alternative(BITS_BUTTON_CLEAR);
                    let token = ctx.alert(opts);
                    self.tokens.insert(token, self.counter);
                    if self.counter < 5 {
                        self.timer_token = ctx.request_timer(Duration::from_millis(200));
                    } else {
                        self.counter = 0;
                        self.timer_token = TimerToken::INVALID;
                    }
                }
            }
            Event::AlertResponse(response) => {
                let bit = self.tokens.get(&response.token()).unwrap();
                if response.button() == Some(&BITS_BUTTON_SET) {
                    data.bits |= 1 << (bit - 1);
                } else if response.button() == Some(&BITS_BUTTON_CLEAR) {
                    data.bits &= !(1 << (bit - 1));
                }
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}
