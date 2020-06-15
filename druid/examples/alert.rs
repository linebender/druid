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
use druid::widget::{Button, Checkbox, Controller, Flex, Label, MainAxisAlignment, Padding};
use druid::{AlertButton, AlertOptions, AlertToken, AppLauncher, WidgetExt, WindowDesc};
use druid::{Application, Data, Lens, TimerToken};

// If you know the alert button label at compile time, you can make const AlertButtons.
const ALERT_BUTTON_TO_LEFT: AlertButton = AlertButton::new("Increase left");
const ALERT_BUTTON_TO_RIGHT: AlertButton = AlertButton::new("Increase right");

#[derive(Debug, Clone, Data, Lens)]
struct State {
    app_modal: bool,
    left: usize,
    right: usize,
    bits: usize,
}

struct ManageButtonController;

struct BitsButtonController {
    button_set: AlertButton,
    button_clear: AlertButton,
    counter: usize,
    timer_token: TimerToken,
    tokens: HashMap<AlertToken, usize>,
}

struct QuitButtonController {
    counter: usize,
    timer_token: TimerToken,
}

const WINDOW_TITLE: &str = "Alerts everywhere";

fn main() {
    let main_window = WindowDesc::new(ui_builder).title(WINDOW_TITLE);
    let state = State {
        app_modal: false,
        left: 5,
        right: 5,
        bits: 0,
    };
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(state)
        .expect("launch failed");
}

fn opts_modal(options: AlertOptions, data: &State) -> AlertOptions {
    if data.app_modal {
        options.app_modal()
    } else {
        options
    }
}

fn ui_builder() -> impl Widget<State> {
    let button_new_window = Button::new("New window").on_click(|ctx, _data, _env| {
        let new_window = WindowDesc::new(ui_builder).title(WINDOW_TITLE);
        ctx.new_window(new_window);
    });
    let button_kitchen = Button::new("Kitchen sink").on_click(|ctx, data, _env| {
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
            .cancel(AlertButton::new("Translated cancel"));
        let opts = opts_modal(opts, data);
        ctx.alert(opts);
    });
    let button_flood = Button::new("Three in a row").on_click(|ctx, data, _env| {
        for i in 0..3 {
            let opts = AlertOptions::new().message(format!("Alert #{}", i + 1));
            let opts = opts_modal(opts, data);
            ctx.alert(opts);
        }
    });
    let button_manage = Button::<State>::new("Manage score")
        .on_click(|ctx, data, _env| {
            let opts = AlertOptions::information()
                .context("Manage score")
                .message("Which side should be increased?")
                .primary(ALERT_BUTTON_TO_LEFT)
                .alternative(ALERT_BUTTON_TO_RIGHT)
                .cancelable();
            let opts = opts_modal(opts, data);
            ctx.alert(opts);
        })
        .controller(ManageButtonController);
    let button_bits = Button::<State>::dynamic(|data, _| format!("Bits: {:05b}", data.bits))
        .controller(BitsButtonController::new("bit"));
    let button_quit = Button::<State>::new("Quit in style").controller(QuitButtonController::new());

    let label_left = Label::new(|data: &State, _: &_| format!("{}", data.left));
    let label_right = Label::new(|data: &State, _: &_| format!("{}", data.right));

    let checkbox_app_modal = Checkbox::new("App modal").lens(State::app_modal);

    let bottom_row = Flex::row()
        .must_fill_main_axis(true)
        .main_axis_alignment(MainAxisAlignment::SpaceBetween)
        .with_child(label_left)
        .with_child(checkbox_app_modal)
        .with_child(label_right);

    Flex::column()
        .main_axis_alignment(MainAxisAlignment::End)
        .with_flex_child(Padding::new(5.0, button_new_window), 1.0)
        .with_flex_child(Padding::new(5.0, button_kitchen), 1.0)
        .with_flex_child(Padding::new(5.0, button_manage), 1.0)
        .with_flex_child(Padding::new(5.0, button_flood), 1.0)
        .with_flex_child(Padding::new(5.0, button_bits), 1.0)
        .with_flex_child(Padding::new(5.0, button_quit), 1.0)
        .with_flex_child(Padding::new(20.0, bottom_row), 1.0)
}

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
                if response.button() == Some(&ALERT_BUTTON_TO_LEFT) && data.right > 0 {
                    data.left += 1;
                    data.right -= 1;
                } else if response.button() == Some(&ALERT_BUTTON_TO_RIGHT) && data.left > 0 {
                    data.left -= 1;
                    data.right += 1;
                }
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}

impl BitsButtonController {
    pub fn new(what: &str) -> BitsButtonController {
        BitsButtonController {
            button_set: AlertButton::dynamic(format!("Set {}", what)),
            button_clear: AlertButton::dynamic(format!("Clear {}", what)),
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
                    let opts = AlertOptions::error()
                        .message(format!("What about bit #{}?", self.counter))
                        .primary(self.button_set.clone())
                        .alternative(self.button_clear.clone());
                    let opts = opts_modal(opts, data);
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
                if response.button() == Some(&self.button_set) {
                    data.bits |= 1 << (bit - 1);
                } else if response.button() == Some(&self.button_clear) {
                    data.bits &= !(1 << (bit - 1));
                }
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}

impl QuitButtonController {
    pub fn new() -> QuitButtonController {
        QuitButtonController {
            counter: 0,
            timer_token: TimerToken::INVALID,
        }
    }
}

impl<W: Widget<State>> Controller<State, W> for QuitButtonController {
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
                    self.counter += 1;
                    if self.counter == 4 {
                        Application::global().quit();
                    } else {
                        let opts = match self.counter {
                            1 => AlertOptions::information().message("Things are getting shaky"),
                            2 => AlertOptions::warning().message("It's about to blow!"),
                            3 => AlertOptions::error().message("Abandon deck!"),
                            _ => unreachable!(),
                        };
                        let opts = opts_modal(opts, data);
                        ctx.alert(opts);
                        self.timer_token = ctx.request_timer(Duration::from_millis(1000));
                    }
                }
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}
