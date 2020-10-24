// Copyright 2018 The Druid Authors.
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

//! This is a simple calculator, it calculates things for you.
//! There is not really a central idea behind this example, it is
//! just here to show you how you could do this. There are other
//! ways of doing this. For example you could have the display be
//! a custom widget, and instead of modifying the `Data` you send
//! commands to the display widget.

use druid::widget::prelude::*;
use druid::widget::{Flex, Label, Painter};
use druid::{theme, AppLauncher, Color, Data, Lens, LocalizedString, WidgetExt, WindowDesc};

fn build_calc() -> impl Widget<CalcState> {
    // We have 5 rows that need to be displayed in a grid,
    // and one display at the top. Here we first declare the
    // display and 5 rows, then we put it together in another Flex
    let display = Label::new(|data: &String, _env: &_| data.clone())
        .with_text_size(32.0)
        .lens(CalcState::value)
        .padding(5.0);

    let row_0 = Flex::row()
        .with_flex_child(op_button_label('c', "CE".to_string()), 1.0)
        .with_spacer(1.0)
        .with_flex_child(op_button('C'), 1.0)
        .with_spacer(1.0)
        .with_flex_child(op_button('⌫'), 1.0)
        .with_spacer(1.0)
        .with_flex_child(op_button('÷'), 1.0);
    let row_1 = Flex::row()
        .with_flex_child(digit_button(7), 1.0)
        .with_spacer(1.0)
        .with_flex_child(digit_button(8), 1.0)
        .with_spacer(1.0)
        .with_flex_child(digit_button(9), 1.0)
        .with_spacer(1.0)
        .with_flex_child(op_button('×'), 1.0);
    let row_2 = Flex::row()
        .with_flex_child(digit_button(4), 1.0)
        .with_spacer(1.0)
        .with_flex_child(digit_button(5), 1.0)
        .with_spacer(1.0)
        .with_flex_child(digit_button(6), 1.0)
        .with_spacer(1.0)
        .with_flex_child(op_button('−'), 1.0);
    let row_3 = Flex::row()
        .with_flex_child(digit_button(1), 1.0)
        .with_spacer(1.0)
        .with_flex_child(digit_button(2), 1.0)
        .with_spacer(1.0)
        .with_flex_child(digit_button(3), 1.0)
        .with_spacer(1.0)
        .with_flex_child(op_button('+'), 1.0);
    let row_4 = Flex::row()
        .with_flex_child(op_button('±'), 1.0)
        .with_spacer(1.0)
        .with_flex_child(digit_button(0), 1.0)
        .with_spacer(1.0)
        .with_flex_child(op_button('.'), 1.0)
        .with_spacer(1.0)
        .with_flex_child(op_button('='), 1.0);
    Flex::column()
        .with_flex_spacer(0.2)
        .with_child(display)
        .with_flex_spacer(0.2)
        .with_flex_child(row_0, 1.0)
        .with_spacer(1.0)
        .with_flex_child(row_1, 1.0)
        .with_spacer(1.0)
        .with_flex_child(row_2, 1.0)
        .with_spacer(1.0)
        .with_flex_child(row_3, 1.0)
        .with_spacer(1.0)
        .with_flex_child(row_4, 1.0)
}

fn digit_button(digit: u8) -> impl Widget<CalcState> {
    let painter = Painter::new(|ctx, _, env| {
        let bounds = ctx.size().to_rect();

        ctx.fill(bounds, &env.get(theme::BACKGROUND_LIGHT));

        if ctx.is_hot() {
            ctx.stroke(bounds.inset(-0.5), &Color::WHITE, 1.0);
        }

        if ctx.is_active() {
            ctx.fill(bounds, &Color::rgb8(0x71, 0x71, 0x71));
        }
    });

    Label::new(format!("{}", digit))
        .with_text_size(24.)
        .center()
        .background(painter)
        .expand()
        .on_click(move |_ctx, data: &mut CalcState, _env| data.digit(digit))
}

fn op_button(op: char) -> impl Widget<CalcState> {
    op_button_label(op, op.to_string())
}

fn op_button_label(op: char, label: String) -> impl Widget<CalcState> {
    let painter = Painter::new(|ctx, _, env| {
        let bounds = ctx.size().to_rect();

        ctx.fill(bounds, &env.get(theme::PRIMARY_DARK));

        if ctx.is_hot() {
            ctx.stroke(bounds.inset(-0.5), &Color::WHITE, 1.0);
        }

        if ctx.is_active() {
            ctx.fill(bounds, &env.get(theme::PRIMARY_LIGHT));
        }
    });

    Label::new(label)
        .with_text_size(24.)
        .center()
        .background(painter)
        .expand()
        .on_click(move |_ctx, data: &mut CalcState, _env| data.op(op))
}

pub fn main() {
    let window = WindowDesc::new(build_calc)
        .window_size((223., 300.))
        .resizable(false)
        .title(LocalizedString::new("Simple Calculator"));
    let calc_state = CalcState {
        value: "0".to_string(),
        operand: 0.0,
        operand2: 0.0,
        operator: 'C',
        post_period: false,
        numbers_after_period: 0,
    };
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(calc_state)
        .expect("launch failed");
}

#[derive(Clone, Data, Lens)]
struct CalcState {
    /// The number displayed. Generally a valid float.
    value: String,
    operand: f64,
    operand2: f64,
    operator: char,
    post_period: bool,
    numbers_after_period: u32,
}

// This is not a very good implementation, but it is a simple one.
// For example 5.999 will be displayed as 5.99900000... because of
// floating point rounding.
impl CalcState {
    fn digit(&mut self, digit: u8) {
        if self.post_period {
            self.numbers_after_period += 1;
            self.operand2 += digit as f64 / (10 as i64).pow(self.numbers_after_period) as f64
        } else {
            self.operand2 *= 10.0;
            self.operand2 += digit as f64;
        }
        self.display2();
    }

    fn display(&mut self) {
        self.value = self.operand.to_string();
    }

    fn display2(&mut self) {
        self.value = self.operand2.to_string();
        if self.post_period && self.numbers_after_period == 0 {
            self.value.push('.');
        }
    }

    fn op(&mut self, op: char) {
        match op {
            '+' | '−' | '×' | '÷' | '=' => {
                let result = match self.operator {
                    '+' => self.operand + self.operand2,
                    '−' => self.operand - self.operand2,
                    '×' => self.operand * self.operand2,
                    '÷' => self.operand / self.operand2,
                    _ => self.operand2,
                };
                self.operand = result;
                self.operand2 = 0.0;
                self.display();
                self.operator = op;
                self.post_period = false;
            }
            '±' => {
                self.operand2 = -self.operand2;
                self.display2();
            }
            '.' => {
                if !self.post_period {
                    self.post_period = true;
                    self.numbers_after_period = 0;
                    self.display2();
                }
            }
            'c' => {
                self.operand = 0.0;
                self.operand2 = 0.0;
                self.display2();
                self.post_period = false;
                self.numbers_after_period = 0;
            }
            'C' => {
                self.operand = 0.0;
                self.operand2 = 0.0;
                self.display2();
                self.operator = 'C';
                self.post_period = false;
                self.numbers_after_period = 0;
            }
            '⌫' => {
                self.value.pop();
                self.operand2 = self.value.parse().unwrap_or(0.0);
                if self.post_period {
                    if self.numbers_after_period == 0 {
                        self.post_period = false;
                    } else {
                        self.numbers_after_period -= 1;
                    }
                }
                self.display2();
            }
            _ => {}
        }
    }
}
