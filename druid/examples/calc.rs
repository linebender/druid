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

use druid::{
    AppLauncher, BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, Lens, LensWrap,
    PaintCtx, UpdateCtx, Widget, WindowDesc,
};

use druid::kurbo::Size;
use druid::widget::{Button, DynLabel, Flex, Padding};

#[derive(Clone, Data, Lens)]
struct CalcState {
    /// The number displayed. Generally a valid float.
    value: String,
    operand: f64,
    operator: char,
    in_num: bool,
}

#[derive(PartialEq, Eq, Debug)]
enum CalcInput {
    Op(char),
    Digit(u8),
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

    fn handle_input(&mut self, input: CalcInput) {
        use CalcInput::*;
        match input {
            Op(c) => self.op(c),
            Digit(d) => self.digit(d),
        }
    }
}

fn pad<T: Data>(inner: impl Widget<T> + 'static) -> impl Widget<T> {
    Padding::new(5.0, inner)
}

fn op_button_label(op: char, label: String) -> impl Widget<CalcState> {
    pad(Button::new(
        label,
        move |_ctx, data: &mut CalcState, _env| data.op(op),
    ))
}

fn op_button(op: char) -> impl Widget<CalcState> {
    op_button_label(op, op.to_string())
}

fn digit_button(digit: u8) -> impl Widget<CalcState> {
    pad(Button::new(
        format!("{}", digit),
        move |_ctx, data: &mut CalcState, _env| data.digit(digit),
    ))
}

fn flex_row<T: Data>(
    w1: impl Widget<T> + 'static,
    w2: impl Widget<T> + 'static,
    w3: impl Widget<T> + 'static,
    w4: impl Widget<T> + 'static,
) -> impl Widget<T> {
    let mut row = Flex::row();
    row.add_child(w1, 1.0);
    row.add_child(w2, 1.0);
    row.add_child(w3, 1.0);
    row.add_child(w4, 1.0);
    row
}

struct KeyboardHandler<T: Data> {
    child: Flex<T>,
}

impl Widget<CalcState> for KeyboardHandler<CalcState> {
    fn paint(
        &mut self,
        paint_ctx: &mut PaintCtx,
        base_state: &BaseState,
        data: &CalcState,
        env: &Env,
    ) {
        self.child.paint(paint_ctx, base_state, data, env)
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &CalcState,
        env: &Env,
    ) -> Size {
        self.child.layout(layout_ctx, bc, data, env)
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut CalcState, env: &Env) {
        use druid::HotKey;
        use druid::KeyCode::*;
        use druid::RawMods;
        use druid::RawMods::Shift;
        use druid_shell::UnknownKeyMap;
        use CalcInput::*;

        match event {
            Event::KeyDown(key_event) => {
                // map to physical keys since we want numbers with numlock both on and off.
                let phys_key = key_event.key_code.to_physical();

                if key_event.is_repeat || !phys_key.is_printable() || phys_key == Space {
                    return;
                }

                let calc_input = match (key_event.mods, phys_key) {
                    key if (HotKey::new(Shift, Key8) == key
                        || HotKey::new(None, NumpadMultiply) == key) =>
                    {
                        Some(Op('×'))
                    }
                    key if (HotKey::new(Shift, Equals) == key
                        || HotKey::new(None, NumpadAdd) == key) =>
                    {
                        Some(Op('+'))
                    }
                    key if (HotKey::new(Shift, KeyC) == key) => Some(Op('C')),
                    // TODO Not really sure a good key for this.
                    key if (HotKey::new(Shift, Backtick) == key) => Some(Op('±')),
                    (mods, key_code) if mods == RawMods::None => match key_code {
                        Numpad0 | Key0 => Some(Digit(0)),
                        Numpad1 | Key1 => Some(Digit(1)),
                        Numpad2 | Key2 => Some(Digit(2)),
                        Numpad3 | Key3 => Some(Digit(3)),
                        Numpad4 | Key4 => Some(Digit(4)),
                        Numpad5 | Key5 => Some(Digit(5)),
                        Numpad6 | Key6 => Some(Digit(6)),
                        Numpad7 | Key7 => Some(Digit(7)),
                        Numpad8 | Key8 => Some(Digit(8)),
                        Numpad9 | Key9 => Some(Digit(9)),
                        KeyC => Some(Op('c')),
                        Backspace => Some(Op('⌫')),
                        NumpadDecimal | Period => Some(Op('.')),
                        NumpadEnter | NumpadEquals | Equals | Return => Some(Op('=')),
                        NumpadSubtract | Minus => Some(Op('−')),
                        Slash | NumpadDivide => Some(Op('÷')),
                        _ => None,
                    },
                    _ => None,
                };

                match calc_input {
                    Some(calc_input) => data.handle_input(calc_input),
                    None => match key_event.text() {
                        Some(text) => log::warn!("Unrecognized input {:?}", text),
                        None => log::warn!("Unrecognized key_code: {:?}", key_event.key_code),
                    },
                }
            }
            // Without focus we won't receive any key events.
            // On startup the root window will receive a Size event,
            Event::Size(_) => {
                // The root window doesn't seem to propagate the size event down the tree.
                // So we don't either.
                ctx.request_focus()
            }
            _ => self.child.event(ctx, event, data, env),
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: Option<&CalcState>,
        data: &CalcState,
        env: &Env,
    ) {
        self.child.update(ctx, old_data, data, env)
    }
}

fn build_calc() -> impl Widget<CalcState> {
    let mut column = Flex::column();
    let display = LensWrap::new(
        DynLabel::new(|data: &String, _env| data.clone()),
        CalcState::value,
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
    KeyboardHandler { child: column }
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
