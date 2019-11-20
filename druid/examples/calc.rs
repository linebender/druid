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

use druid::widget::{Button, Column, DynLabel, Flex, Padding, Row};
use druid::{AppLauncher, Data, Lens, LensWrap, Widget, WindowDesc};
use druid::{BaseState, BoxConstraints, Env, EventCtx, LayoutCtx, PaintCtx, UpdateCtx};
use druid::{Event, KeyCode, KeyEvent, KeyModifiers};
use druid_shell::kurbo::Size;
use std::collections::HashMap;

#[derive(Clone, Data, Lens)]
struct CalcState {
    /// The number displayed. Generally a valid float.
    value: String,
    operand: f64,
    operator: char,
    in_num: bool,
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

#[derive(Debug)]
struct ButtonInfo {
    row_id: usize,
    col_id: usize,
}

#[derive(PartialEq, Eq, Hash, Debug)]
enum ButtonId {
    Op(char),
    Digit(u8),
}

struct KeyMap {
    // Note we rely on the fact that the key_map has row and column ids
    // into the Flex, so the flex here cannot be dynamically changing.
    key_map: HashMap<ButtonId, ButtonInfo>,
    flex: Flex<CalcState>,
}

impl KeyMap {
    fn new(flex: Flex<CalcState>, key_map: HashMap<ButtonId, ButtonInfo>) -> KeyMap {
        KeyMap { key_map, flex }
    }
}

fn key_event_to_button_id(event: &KeyEvent) -> Option<ButtonId> {
    use crate::ButtonId::*;
    match event.mods {
        KeyModifiers {
            shift: true,
            alt: false,
            meta: false,
            ctrl: false,
        } => match event.key_code {
            KeyCode::Key8 => Some(Op('×')),
            KeyCode::Equals => Some(Op('+')),
            KeyCode::KeyC => Some(Op('C')),
            KeyCode::Backtick => Some(Op('±')),
            _ => None,
        },
        KeyModifiers {
            shift: false,
            alt: false,
            meta: false,
            ctrl: false,
        } => match event.key_code {
            KeyCode::Numpad0 | KeyCode::Key0 => Some(Digit(0)),
            KeyCode::Numpad1 | KeyCode::Key1 => Some(Digit(1)),
            KeyCode::Numpad2 | KeyCode::Key2 => Some(Digit(2)),
            KeyCode::Numpad3 | KeyCode::Key3 => Some(Digit(3)),
            KeyCode::Numpad4 | KeyCode::Key4 => Some(Digit(4)),
            KeyCode::Numpad5 | KeyCode::Key5 => Some(Digit(5)),
            KeyCode::Numpad6 | KeyCode::Key6 => Some(Digit(6)),
            KeyCode::Numpad7 | KeyCode::Key7 => Some(Digit(7)),
            KeyCode::Numpad8 | KeyCode::Key8 => Some(Digit(8)),
            KeyCode::Numpad9 | KeyCode::Key9 => Some(Digit(9)),
            KeyCode::NumpadSubtract | KeyCode::Minus => Some(Op('−')),
            KeyCode::NumpadAdd => Some(Op('+')),
            KeyCode::NumpadMultiply => Some(Op('×')),
            KeyCode::NumpadEnter | KeyCode::NumpadEquals | KeyCode::Equals | KeyCode::Return => {
                Some(Op('='))
            }
            KeyCode::Slash | KeyCode::NumpadDivide => Some(Op('÷')),
            KeyCode::KeyC => Some(Op('c')),
            KeyCode::Backspace => Some(Op('⌫')),
            KeyCode::NumpadDecimal | KeyCode::Period => Some(Op('.')),
            _ => None,
        },
        _ => None,
    }
}

impl Widget<CalcState> for KeyMap {
    fn paint(
        &mut self,
        paint_ctx: &mut PaintCtx,
        base_state: &BaseState,
        data: &CalcState,
        env: &Env,
    ) {
        self.flex.paint(paint_ctx, base_state, data, env)
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &CalcState,
        env: &Env,
    ) -> Size {
        self.flex.layout(layout_ctx, bc, data, env)
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut CalcState, env: &Env) {
        use crate::ButtonId::*;
        match event {
            Event::KeyDown(key_event) => {
                if key_event.is_repeat {
                    return;
                };

                let kc = key_event_to_button_id(key_event);
                // Current we can just perform the action on the CalcState,
                // However we don't get the pretty button animation this way.
                match kc {
                    None => self.flex.event(ctx, event, data, env),
                    Some(Op(c)) => data.op(c),
                    Some(Digit(d)) => data.digit(d),
                }
                match kc {
                    None => (),
                    Some(button_id) => {
                        let ButtonInfo {
                            row_id: _row_id,
                            col_id: _col_id,
                        } = self.key_map.get(&button_id).unwrap();
                        // In theory if there was a mechanism to get random access into
                        // self.flex.children we could remove the above match,
                        // and do something fancy with the buttons here instead.
                        ()
                    }
                }
            }
            Event::Size(_) => ctx.request_focus(),
            _ => self.flex.event(ctx, event, data, env),
        }
    }
    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: Option<&CalcState>,
        data: &CalcState,
        env: &Env,
    ) {
        self.flex.update(ctx, old_data, data, env)
    }
}

fn pad<T: Data>(inner: impl Widget<T> + 'static) -> Padding<T> {
    Padding::new(5.0, inner)
}

fn op_button_label(op: char, label: String) -> Padding<CalcState> {
    pad(Button::new(
        label,
        move |_ctx, data: &mut CalcState, _env| data.op(op),
    ))
}

fn op_button(op: char) -> Padding<CalcState> {
    op_button_label(op, op.to_string())
}

fn digit_button(digit: u8) -> Padding<CalcState> {
    pad(Button::new(
        format!("{}", digit),
        move |_ctx, data: &mut CalcState, _env| data.digit(digit),
    ))
}

struct ButtonLabel(ButtonId, Option<&'static str>);

fn build_calc() -> impl Widget<CalcState> {
    use crate::ButtonId::*;

    let mut column = Column::new();
    let display = LensWrap::new(
        DynLabel::new(|data: &String, _env| data.clone()),
        lenses::calc_state::value,
    );

    let button_rows = [
        [
            ButtonLabel(Op('c'), Some("CE")),
            ButtonLabel(Op('C'), None),
            ButtonLabel(Op('⌫'), None),
            ButtonLabel(Op('÷'), None),
        ],
        [
            ButtonLabel(Digit(7), None),
            ButtonLabel(Digit(8), None),
            ButtonLabel(Digit(9), None),
            ButtonLabel(Op('×'), None),
        ],
        [
            ButtonLabel(Digit(4), None),
            ButtonLabel(Digit(5), None),
            ButtonLabel(Digit(6), None),
            ButtonLabel(Op('−'), None),
        ],
        [
            ButtonLabel(Digit(1), None),
            ButtonLabel(Digit(2), None),
            ButtonLabel(Digit(3), None),
            ButtonLabel(Op('+'), None),
        ],
        [
            ButtonLabel(Digit(0), None),
            ButtonLabel(Op('±'), None),
            ButtonLabel(Op('.'), None),
            ButtonLabel(Op('='), None),
        ],
    ];

    column.add_child(pad(display), 0.0);
    let mut key_map = HashMap::new();
    for (col_id, buttons) in button_rows.iter().enumerate() {
        // Because of the pad(display) above add 1?
        let col_id = col_id + 1;
        let mut row = Row::new();
        for (row_id, button) in buttons.iter().enumerate() {
            match button {
                ButtonLabel(Op(c), None) => {
                    let button = op_button(*c);
                    key_map.insert(Op(*c), ButtonInfo { row_id, col_id });
                    row.add_child(button, 1.0);
                }
                ButtonLabel(Op(c), Some(label)) => {
                    let button = op_button_label(*c, label.to_string());
                    key_map.insert(Op(*c), ButtonInfo { row_id, col_id });
                    row.add_child(button, 1.0);
                }
                ButtonLabel(Digit(d), _) => {
                    let button = digit_button(*d);
                    key_map.insert(Digit(*d), ButtonInfo { row_id, col_id });
                    row.add_child(button, 1.0);
                }
            };
        }
        column.add_child(row, 1.0);
    }
    let key_map_widget = KeyMap::new(column, key_map);
    key_map_widget
}

fn main() {
    let window = WindowDesc::new(build_calc);
    let calc_state = CalcState {
        value: "0".to_string(),
        operand: 0.0,
        operator: 'C',
        in_num: false,
    };
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(calc_state)
        .expect("launch failed");
}
