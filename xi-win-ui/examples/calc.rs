// Copyright 2018 Google LLC
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

extern crate xi_win_shell;
extern crate xi_win_ui;

use std::any::Any;

use xi_win_shell::win_main;
use xi_win_shell::window::WindowBuilder;

use xi_win_ui::{HandlerCtx, UiInner, UiMain, UiState, Widget};
use xi_win_ui::widget::{Button, Column, Label, Row, Padding};

use xi_win_ui::Id;

struct CalcState {
    /// The number displayed. Generally a valid float.
    value: String,
    operand: f64,
    operator: char,
    in_num: bool,
}

impl Widget for CalcState {
    fn poke(&mut self, payload: &mut Any, ctx: &mut HandlerCtx) -> bool {
        if let Some(digit) = payload.downcast_ref::<u8>() {
            self.digit(*digit);
            ctx.send_event(self.value.clone());
            true
        } else if let Some(op) = payload.downcast_ref::<char>() {
            self.op(*op);
            ctx.send_event(self.value.clone());
            true
        } else {
            false
        }
    }
}

impl CalcState {
    pub fn ui(self, ctx: &mut UiInner, child: Id) -> Id {
        ctx.add(self, &[child])
    }

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

fn pad(widget: Id, ui: &mut UiState) -> Id {
    Padding::uniform(5.0).ui(widget, ui)
}

fn digit_button(ui: &mut UiState, mut digit: u8) -> Id
{
    let button = Button::new(digit.to_string()).ui(ui);
    ui.add_listener(button, move |_: &mut bool, mut ctx| {
        ctx.poke_up(&mut digit);
    });
    pad(button, ui)
}

fn op_button_label(ui: &mut UiState, mut op: char, label: String) -> Id
{
    let button = Button::new(label).ui(ui);
    ui.add_listener(button, move |_: &mut bool, mut ctx| {
        ctx.poke_up(&mut op);
    });
    pad(button, ui)
}

fn op_button(ui: &mut UiState, op: char) -> Id
{
    op_button_label(ui, op, op.to_string())
}

fn build_calc(ui: &mut UiState) {
    let display = Label::new("0".to_string()).ui(ui);
    let row0 = pad(display, ui);

    let row1 = Row::new().ui(&[
        op_button_label(ui, 'c', "CE".to_string()),
        op_button(ui, 'C'),
        op_button(ui, '⌫'),
        op_button(ui, '÷'),
    ], ui);
    let row2 = Row::new().ui(&[
        digit_button(ui, 7),
        digit_button(ui, 8),
        digit_button(ui, 9),
        op_button(ui, '×'),
    ], ui);
    let row3 = Row::new().ui(&[
        digit_button(ui, 4),
        digit_button(ui, 5),
        digit_button(ui, 6),
        op_button(ui, '−'),
    ], ui);
    let row4 = Row::new().ui(&[
        digit_button(ui, 1),
        digit_button(ui, 2),
        digit_button(ui, 3),
        op_button(ui, '+'),
    ], ui);
    let row5 = Row::new().ui(&[
        op_button(ui, '±'),
        digit_button(ui, 0),
        op_button(ui, '.'),
        op_button(ui, '='),
    ], ui);
    let panel = Column::new().ui(&[row0, row1, row2, row3, row4, row5], ui);
    let calc_state = CalcState {
        value: "0".to_string(),
        operand: 0.0,
        operator: 'C',
        in_num: false,
    }.ui(ui, panel);
    ui.add_listener(calc_state, move |value: &mut String, mut ctx| {
        ctx.poke(display, value);
    });
    let root = pad(calc_state, ui);
    ui.set_root(root);
}

fn main() {
    xi_win_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();
    build_calc(&mut state);
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Calculator");
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
