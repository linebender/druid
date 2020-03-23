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
    theme, AppLauncher, Color, Data, Lens, LocalizedString, RenderContext, Widget, WidgetExt,
    WindowDesc,
};

use druid::{
    piet::{FixedLinearGradient, GradientStop},
    Point,
};
use druid::{
    widget::{CrossAxisAlignment, Flex, Label, MainAxisAlignment, Padding, Painter},
    UnitPoint,
};

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

fn op_button_label(op: char, label: String) -> impl Widget<CalcState> {
    let painter = Painter::new(|ctx, _, env| {
        let bounds = ctx.size().to_rect();
        let color = env.get(theme::PRIMARY_DARK);
        let color_hot: Color = Color::from_rgba32_u32(color.clone().as_rgba_u32() + 0x303030);
        ctx.fill(bounds, &color);

        if ctx.is_hot() {
            // ctx.stroke(bounds.inset(-0.5), &Color::WHITE, 1.0);
            ctx.fill(bounds, &color_hot);
        }

        if ctx.is_active() {
            ctx.fill(bounds, &env.get(theme::PRIMARY_LIGHT));
        }
    });

    Label::new(label)
        .text_size(24.)
        .center()
        .background(painter)
        .expand()
        .on_click(move |_ctx, data: &mut CalcState, _env| data.op(op))
}

fn op_button(op: char) -> impl Widget<CalcState> {
    op_button_label(op, op.to_string())
}

fn digit_button(digit: u8) -> impl Widget<CalcState> {
    let painter = Painter::new(|ctx, _, env| {
        let bounds = ctx.size().to_rect();
        let color = env.get(theme::BACKGROUND_LIGHT);
        let color_hot: Color = Color::from_rgba32_u32(color.clone().as_rgba_u32() + 0x303030);
        let color_active: Color = Color::from_rgba32_u32(color.clone().as_rgba_u32() + 0x606060);

        ctx.fill(bounds, &color);

        if ctx.is_hot() {
            // ctx.stroke(bounds.inset(-0.5), &Color::WHITE, 1.0);
            ctx.fill(bounds, &color_hot);
        }

        if ctx.is_active() {
            ctx.fill(bounds, &color_active);
        }
    });

    Label::new(format!("{}", digit))
        .text_size(24.)
        .center()
        .background(painter)
        .expand()
        .on_click(move |_ctx, data: &mut CalcState, _env| data.digit(digit))
}

fn flex_row3<T: Data>(
    w1: impl Widget<T> + 'static,
    w2: impl Widget<T> + 'static,
    w3: impl Widget<T> + 'static,
) -> impl Widget<T> {
    Flex::row()
        .with_flex_child(Padding::new(1., w1), 2.0)
        .with_flex_child(Padding::new(1., w2), 1.0)
        .with_flex_child(Padding::new(1., w3), 1.0)
}

fn flex_row4<T: Data>(
    w1: impl Widget<T> + 'static,
    w2: impl Widget<T> + 'static,
    w3: impl Widget<T> + 'static,
    w4: impl Widget<T> + 'static,
) -> impl Widget<T> {
    Flex::row()
        .with_flex_child(Padding::new(1., w1), 1.0)
        .with_flex_child(Padding::new(1., w2), 1.0)
        .with_flex_child(Padding::new(1., w3), 1.0)
        .with_flex_child(Padding::new(1., w4), 1.0)
}

fn build_calc() -> impl Widget<CalcState> {
    let display_painter = Painter::new(|ctx, _, _env| {
        let bounds = ctx.size().to_rect();
        if let Ok(brush) = ctx.gradient(FixedLinearGradient {
            start: Point::new(0., 0.),
            end: Point::new(100., 100.),
            stops: vec![
                GradientStop {
                    pos: 0.0,
                    color: Color::from_rgba32_u32(0x225553ff),
                },
                GradientStop {
                    pos: 1.0,
                    color: Color::from_rgba32_u32(0x113333ff),
                },
            ],
        }) {
            ctx.fill(bounds, &brush);
        }
    });
    let display = Label::new(|data: &String, _env: &_| data.clone())
        .text_size(32.0)
        .lens(CalcState::value)
        .padding(5.0);
    let display_row = Padding::new(
        5.,
        Flex::row()
            .with_flex_child(display, 0.0)
            .main_axis_alignment(MainAxisAlignment::End)
            .expand_width()
            .background(display_painter),
    );
    Flex::column()
        .with_flex_spacer(0.1)
        .with_child(display_row)
        .with_flex_spacer(0.1)
        .cross_axis_alignment(CrossAxisAlignment::End)
        .with_flex_child(
            flex_row3(
                op_button_label('C', "AC".to_string()),
                op_button('⌫'),
                op_button('÷'),
            ),
            1.0,
        )
        .with_spacer(1.0)
        .with_flex_child(
            flex_row4(
                digit_button(7),
                digit_button(8),
                digit_button(9),
                op_button('×'),
            ),
            1.0,
        )
        .with_spacer(1.0)
        .with_flex_child(
            flex_row4(
                digit_button(4),
                digit_button(5),
                digit_button(6),
                op_button('−'),
            ),
            1.0,
        )
        .with_spacer(1.0)
        .with_flex_child(
            flex_row4(
                digit_button(1),
                digit_button(2),
                digit_button(3),
                op_button('+'),
            ),
            1.0,
        )
        .with_spacer(1.0)
        .with_flex_child(
            flex_row4(
                op_button('±'),
                digit_button(0),
                op_button('.'),
                op_button('='),
            ),
            1.0,
        )
}

fn main() {
    let window = WindowDesc::new(build_calc)
        .window_size((223., 300.))
        .resizable(false)
        .title(
            LocalizedString::new("calc-demo-window-title").with_placeholder("Simple Calculator"),
        );
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
