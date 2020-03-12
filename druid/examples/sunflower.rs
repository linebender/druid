// Copyright 2019 The xi-editor Authors.
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

//! Sunflower

use std::f64::consts::{E, PI, SQRT_2};
use std::ops::{Index, IndexMut};
use std::time::{Duration, Instant};

use druid::widget::{Button, CrossAxisAlignment, Flex, Label, Slider, Stepper, WidgetExt, TextBox};
use druid::{
    AppLauncher, BoxConstraints, Color, Data, Env, Event, EventCtx, LayoutCtx, LensExt, Lens, LifeCycle,
    LifeCycleCtx, LocalizedString, MouseButton, PaintCtx, Point, Rect, RenderContext, Size,
    TimerToken, UpdateCtx, Widget, WindowDesc,
};
use druid::kurbo::Circle;
use fluent_syntax::ast::InlineExpression::StringLiteral;

const e_inv: f64 = 1. / E;
const phi_inv: f64 = 1. / 1.61803398875;
const two_sqrt_inv: f64 = 1. / SQRT_2;
const three_sqrt_inv: f64 = 1. / 1.73205080757;
const pi_inv: f64 = 1. / PI;
const default_angle_factor: f64 = 0.256666666666;

#[derive(Clone, Lens, Data)]
struct AppData {
    angle_factor: f64,
    scale: f64,
    n_seeds: f64,
}

struct SunflowerWidget {}

impl Widget<AppData> for SunflowerWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppData, _env: &Env) {}

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &AppData,
        _env: &Env,
    ) {}

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &AppData, _data: &AppData, _env: &Env) {
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &AppData,
        _env: &Env,
    ) -> Size {
        let max_size = bc.max();
        let min_side = max_size.height.min(max_size.width);
        Size {
            width: min_side,
            height: min_side,
        }
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &AppData, _env: &Env) {
        let size: Size = paint_ctx.size();
        let m = size.width as f64 / 2.;
        let center = Point {
            x: m,
            y: m,
        };
        for i in 0..data.n_seeds as u64 {
            let theta = i as f64 * 2. * PI * data.angle_factor;
            let r = (i as f64).sqrt() * data.scale;
            let x = m + r * theta.cos();
            let y = m - r * theta.sin();
            paint_ctx.fill(Circle::new(Point { x, y }, 3.), &Color::rgb8(233 as u8, 233 as u8, 0 as u8));
        }
    }
}

fn make_widget() -> impl Widget<AppData> {
    Flex::column()
        .with_child(SunflowerWidget {}, 1.0)
        .with_child(
            Flex::column()
                .with_child(
                    Label::new(|data: &AppData, _env: &_| {
                        format!("Angle incrementation factor: {:.8}", data.angle_factor)
                    })
                        .padding(3.0),
                    0.,
                )
                .with_child(
                    Flex::row()
                        .with_child(
                            Button::new("Use SQRT2", |ctx, data: &mut f64, _: &Env| {
                                *data = two_sqrt_inv;
                            })
                                .lens(AppData::angle_factor)
                                .padding((5., 5.)),
                            1.0,
                        )
                        .with_child(
                            Button::new("Use SQRT3", |ctx, data: &mut f64, _: &Env| {
                                *data = three_sqrt_inv;
                            })
                                .lens(AppData::angle_factor)
                                .padding((5., 5.)),
                            1.0,
                        )
                        .with_child(
                            Button::new("Use e", |ctx, data: &mut f64, _: &Env| {
                                *data = e_inv;
                            })
                                .lens(AppData::angle_factor)
                                .padding((5., 5.)),
                            1.0,
                        )
                        .with_child(
                            Button::new("Use golden ratio", |ctx, data: &mut f64, _: &Env| {
                                *data = phi_inv;
                            })
                                .lens(AppData::angle_factor)
                                .padding((5., 5.)),
                            1.0,
                        )
                        .padding(8.0),
                    0.,
                )
                .with_child(
                    Flex::row()
                        .with_child(
                            TextBox::new()
                                .with_placeholder(default_angle_factor.to_string())
                                .parse()
                                .lens(AppData::angle_factor.map(
                                    |x| Some(*x),
                                    |x, y| *x = y.unwrap_or(default_angle_factor),
                                )),
                            1.0,
                        )
                        .with_child(
                            Stepper::new()
                                .min(0.)
                                .max(1.)
                                .step(0.00001)
                                .lens(AppData::angle_factor),
                            0.,
                        ),
                    0.,
                )
                .with_child(
                    Label::new(|data: &AppData, _env: &_| {
                        format!("Number of seeds: {}", data.n_seeds)
                    })
                        .padding(3.0),
                    0.,
                )
                .with_child(
                    Flex::row()
                        .with_child(
                            Slider::new()
                                .with_range(10., 5000.0)
                                .lens(AppData::n_seeds)
                                .padding(3.),
                            1.,
                        ),
                    0.,
                )
                .with_child(
                    Label::new(|data: &AppData, _env: &_| {
                        format!("Radius increment scale {}", data.scale)
                    })
                        .padding(3.0),
                    0.,
                )
                .with_child(
                    Flex::row()
                        .with_child(
                            Slider::new()
                                .with_range(4., 10.)
                                .lens(AppData::scale)
                                .padding(3.),
                            1.,
                        ),
                    0.,
                )
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .padding(8.)
                .background(Color::grey(0.2)),
            0.,
        )
        .cross_axis_alignment(CrossAxisAlignment::Center)
}

fn main() {
    let window = WindowDesc::new(make_widget)
        .window_size(Size {
            width: 800.0,
            height: 800.0,
        })
        .resizable(false)
        .title(
            LocalizedString::new("custom-widget-demo-window-title")
                .with_placeholder("Sunflower"),
        );
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(AppData {
            angle_factor: default_angle_factor,
            n_seeds: 600.,
            scale: 4.0,
        })
        .expect("launch failed");
}
