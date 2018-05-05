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

use std::cell::RefCell;
use std::rc::Rc;

use xi_win_shell::win_main;
use xi_win_shell::window::WindowBuilder;

use xi_win_ui::{UiMain, UiState};
use xi_win_ui::widget::{Button, Column, Label, Row, Padding};

use xi_win_ui::Id;

struct CalcState {
    /// The number displayed. Generally a valid float.
    display: String,
    operand: f64,
    operator: char,
    in_num: bool,
}

impl CalcState {
    fn digit(&mut self, digit: u8) {
        if !self.in_num {
            self.display.clear();
            self.in_num = true;
        }
        let ch = (b'0' + digit) as char;
        self.display.push(ch);
    }

    fn display(&mut self) {
        // TODO: change hyphen-minus to actual minus
        self.display = self.operand.to_string();
    }

    fn compute(&mut self) {
        if self.in_num {
            let operand2 = self.display.parse().unwrap_or(0.0);
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
                self.operand = self.display.parse().unwrap_or(0.0);
                self.operator = op;
                self.in_num = false;
            }
            '±' => {
                if self.in_num {
                    if self.display.starts_with('−') {
                        self.display = self.display[3..].to_string();
                    } else {
                        self.display = ["−", &self.display].concat();
                    }
                } else {
                    self.operand = -self.operand;
                    self.display();
                }
            }
            '.' => {
                if !self.in_num {
                    self.display = "0".to_string();
                    self.in_num = true;
                }
                if self.display.find('.').is_none() {
                    self.display.push('.');
                }
            }
            'c' => {
                self.display = "0".to_string();
                self.in_num = false;
            }
            'C' => {
                self.display = "0".to_string();
                self.operator = 'C';
                self.in_num = false;
            }
            '⌫' => {
                if self.in_num {
                    self.display.pop();
                    if self.display.is_empty() || self.display == "−" {
                        self.display = "0".to_string();
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

fn digit_button(ui: &mut UiState, display: Id, calc: &Rc<RefCell<CalcState>>,
    digit: u8) -> Id
{
    let button = Button::new(digit.to_string()).ui(ui);
    let calc = calc.clone();
    ui.add_listener(button, move |_: &mut bool, mut ctx| {
        let mut calc = calc.borrow_mut();
        calc.digit(digit);
        ctx.poke(display, &mut calc.display);
    });
    pad(button, ui)
}

fn op_button_label(ui: &mut UiState, display: Id, calc: &Rc<RefCell<CalcState>>,
    op: char, label: String) -> Id
{
    let button = Button::new(label).ui(ui);
    let calc = calc.clone();
    ui.add_listener(button, move |_: &mut bool, mut ctx| {
        let mut calc = calc.borrow_mut();
        calc.op(op);
        ctx.poke(display, &mut calc.display);
    });
    pad(button, ui)
}

fn op_button(ui: &mut UiState, display: Id, calc: &Rc<RefCell<CalcState>>,
    op: char) -> Id
{
    op_button_label(ui, display, calc, op, op.to_string())
}

fn build_calc(ui: &mut UiState) {
    let calc = Rc::new(RefCell::new(CalcState {
        display: "0".to_string(),
        operand: 0.0,
        operator: 'C',
        in_num: false,
    }));

    let display = Label::new(calc.borrow().display.clone()).ui(ui);
    let row0 = pad(display, ui);

    let row1 = Row::new().ui(&[
        op_button_label(ui, display, &calc, 'c', "CE".to_string()),
        op_button(ui, display, &calc, 'C'),
        op_button(ui, display, &calc, '⌫'),
        op_button(ui, display, &calc, '÷'),
    ], ui);
    let row2 = Row::new().ui(&[
        digit_button(ui, display, &calc, 7),
        digit_button(ui, display, &calc, 8),
        digit_button(ui, display, &calc, 9),
        op_button(ui, display, &calc, '×'),
    ], ui);
    let row3 = Row::new().ui(&[
        digit_button(ui, display, &calc, 4),
        digit_button(ui, display, &calc, 5),
        digit_button(ui, display, &calc, 6),
        op_button(ui, display, &calc, '−'),
    ], ui);
    let row4 = Row::new().ui(&[
        digit_button(ui, display, &calc, 1),
        digit_button(ui, display, &calc, 2),
        digit_button(ui, display, &calc, 3),
        op_button(ui, display, &calc, '+'),
    ], ui);
    let row5 = Row::new().ui(&[
        op_button(ui, display, &calc, '±'),
        digit_button(ui, display, &calc, 0),
        op_button(ui, display, &calc, '.'),
        op_button(ui, display, &calc, '='),
    ], ui);
    let panel = Column::new().ui(&[row0, row1, row2, row3, row4, row5], ui);
    let root = pad(panel, ui);
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
