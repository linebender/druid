// Copyright 2018 The xi-editor Authors.
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

//! Simple calculator.

use druid::shell::{runloop, WindowBuilder};
use druid::{Data, LensWrap, UiMain, UiState, Widget};

use druid::widget::{ActionWrapper, Button, Column, DynLabel, Padding, Row};

#[derive(Clone)]
struct CalcState {
    /// The number displayed. Generally a valid float.
    value: String,
    operand: f64,
    operator: char,
    in_num: bool,
}

// All this should be produced by a derive macro.
mod lenses {
    // Discussion: if the inner type were listed first, then
    // the capitalization wouldn't have to be twizzled.
    pub mod calc_state {
        use super::super::CalcState;
        use druid::Lens;
        pub struct Value;

        impl Lens<CalcState, String> for Value {
            fn get<'a>(&self, data: &'a CalcState) -> &'a String {
                &data.value
            }

            fn with_mut<V, F: FnOnce(&mut String) -> V>(&self, data: &mut CalcState, f: F) -> V {
                f(&mut data.value)
            }
        }
    }
}

// It should be able to get this from a derive macro.
impl Data for CalcState {
    fn same(&self, other: &Self) -> bool {
        self.value.same(&other.value)
            && self.operand.same(&other.operand)
            && self.operator.same(&other.operator)
            && self.in_num.same(&other.in_num)
    }
}

impl CalcState {
    fn digit(&mut self, digit: u8) {
        if !self.in_num {
            self.value.clear();
            self.in_num = true;
        }
        let ch = (b'0' + digit) as char;
        self.value.push(ch);
    }

    fn display(&mut self) {
        // TODO: change hyphen-minus to actual minus
        self.value = self.operand.to_string();
    }

    fn compute(&mut self) {
        if self.in_num {
            let operand2 = self.value.parse().unwrap_or(0.0);
            let result = match self.operator {
                '+' => Some(self.operand + operand2),
                '−' => Some(self.operand - operand2),
                '×' => Some(self.operand * operand2),
                '÷' => Some(self.operand / operand2),
                _ => None,
            };
            if let Some(result) = result {
                self.operand = result;
                self.display();
                self.in_num = false;
            }
        }
    }

    fn op(&mut self, op: char) {
        match op {
            '+' | '−' | '×' | '÷' | '=' => {
                self.compute();
                self.operand = self.value.parse().unwrap_or(0.0);
                self.operator = op;
                self.in_num = false;
            }
            '±' => {
                if self.in_num {
                    if self.value.starts_with('−') {
                        self.value = self.value[3..].to_string();
                    } else {
                        self.value = ["−", &self.value].concat();
                    }
                } else {
                    self.operand = -self.operand;
                    self.display();
                }
            }
            '.' => {
                if !self.in_num {
                    self.value = "0".to_string();
                    self.in_num = true;
                }
                if self.value.find('.').is_none() {
                    self.value.push('.');
                }
            }
            'c' => {
                self.value = "0".to_string();
                self.in_num = false;
            }
            'C' => {
                self.value = "0".to_string();
                self.operator = 'C';
                self.in_num = false;
            }
            '⌫' => {
                if self.in_num {
                    self.value.pop();
                    if self.value.is_empty() || self.value == "−" {
                        self.value = "0".to_string();
                        self.in_num = false;
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

fn pad<T: Data>(inner: impl Widget<T> + 'static) -> impl Widget<T> {
    Padding::uniform(5.0, inner)
}

fn op_button_label(op: char, label: String) -> impl Widget<CalcState> {
    pad(ActionWrapper::new(
        Button::new(label),
        move |data: &mut CalcState, _env| data.op(op),
    ))
}

fn op_button(op: char) -> impl Widget<CalcState> {
    op_button_label(op, op.to_string())
}

fn digit_button(digit: u8) -> impl Widget<CalcState> {
    pad(ActionWrapper::new(
        Button::new(format!("{}", digit)),
        move |data: &mut CalcState, _env| data.digit(digit),
    ))
}

fn flex_row<T: Data>(
    w1: impl Widget<T> + 'static,
    w2: impl Widget<T> + 'static,
    w3: impl Widget<T> + 'static,
    w4: impl Widget<T> + 'static,
) -> impl Widget<T> {
    let mut row = Row::new();
    row.add_child(w1, 1.0);
    row.add_child(w2, 1.0);
    row.add_child(w3, 1.0);
    row.add_child(w4, 1.0);
    row
}

fn build_calc() -> impl Widget<CalcState> {
    let mut column = Column::new();
    let display = LensWrap::new(
        DynLabel::new(|data: &String, _env| data.clone()),
        lenses::calc_state::Value,
    );
    column.add_child(pad(display), 0.0);
    column.add_child(
        flex_row(
            op_button_label('c', "CE".to_string()),
            op_button('C'),
            op_button('⌫'),
            op_button('÷'),
        ),
        1.0,
    );
    column.add_child(
        flex_row(
            digit_button(7),
            digit_button(8),
            digit_button(9),
            op_button('×'),
        ),
        1.0,
    );
    column.add_child(
        flex_row(
            digit_button(4),
            digit_button(5),
            digit_button(6),
            op_button('−'),
        ),
        1.0,
    );
    column.add_child(
        flex_row(
            digit_button(1),
            digit_button(2),
            digit_button(3),
            op_button('+'),
        ),
        1.0,
    );
    column.add_child(
        flex_row(
            op_button('±'),
            digit_button(0),
            op_button('.'),
            op_button('='),
        ),
        1.0,
    );
    column
}

fn main() {
    druid_shell::init();

    let mut run_loop = runloop::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let root = build_calc();
    let calc_state = CalcState {
        value: "0".to_string(),
        operand: 0.0,
        operator: 'C',
        in_num: false,
    };
    let state = UiState::new(root, calc_state);
    builder.set_title("Calculator");
    builder.set_handler(Box::new(UiMain::new(state)));
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
