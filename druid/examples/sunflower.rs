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

use druid::kurbo::Circle;
use druid::widget::{Button, CrossAxisAlignment, Flex, Label, Stepper, TextBox, WidgetExt};
use druid::{
    AppLauncher, BoxConstraints, Color, Data, Env, Event, EventCtx, LayoutCtx, Lens, LensExt,
    LifeCycle, LifeCycleCtx, LocalizedString, PaintCtx, Point, RenderContext, Size, UpdateCtx,
    Widget, WindowDesc,
};

const E_INV: f64 = 1. / E;
const PHI_INV: f64 = 1. / 1.61803398875;
const TWO_SQRT_INV: f64 = 1. / SQRT_2;
const THREE_SQRT_INV: f64 = 1. / 1.73205080757;
const DEFAULT_ANGLE_FACTOR: f64 = 0.256666666666;
const DEFAULT_N_SEEDS: f64 = 1000.;
const DEFAULT_RADIUS_SCALE: f64 = 6.;

#[derive(Clone, Lens, Data)]
struct AppData {
    angle_factor: f64,
    scale: f64,
    n_seeds: f64,
}

struct SunflowerWidget {}

impl Widget<AppData> for SunflowerWidget {
    fn event(&mut self, _ctx: &mut EventCtx, _eventt: &Event, _data: &mut AppData, _env: &Env) {}

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &AppData,
        _env: &Env,
    ) {
    }

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
        for i in 0..data.n_seeds as u64 {
            let theta = i as f64 * 2. * PI * data.angle_factor;
            let r = (i as f64).sqrt() * data.scale;
            let x = m + r * theta.cos();
            let y = m - r * theta.sin();
            paint_ctx.fill(
                Circle::new(Point { x, y }, 3.),
                &Color::rgb8(233 as u8, 233 as u8, 0 as u8),
            );
        }
    }
}

fn make_widget() -> impl Widget<AppData> {
    Flex::column()
        .with_child(SunflowerWidget {}, 1.0)
        .with_child(
            Flex::row()
                .with_child(
                    // buttons on the left
                    Flex::column()
                        .with_child(Label::new("Angle factor shortcuts").padding(10.), 0.)
                        .with_child(
                            Flex::row().with_child(
                                Button::new("Use SQRT2", |_ctx, data: &mut f64, _: &Env| {
                                    *data = TWO_SQRT_INV;
                                })
                                .lens(AppData::angle_factor)
                                .padding((5., 5.)),
                                1.,
                            ),
                            0.0,
                        )
                        .with_child(
                            Flex::row().with_child(
                                Button::new("Use SQRT3", |_ctx, data: &mut f64, _: &Env| {
                                    *data = THREE_SQRT_INV;
                                })
                                .lens(AppData::angle_factor)
                                .padding((5., 5.)),
                                1.,
                            ),
                            0.0,
                        )
                        .with_child(
                            Flex::row().with_child(
                                Button::new("Use e", |_ctx, data: &mut f64, _: &Env| {
                                    *data = E_INV;
                                })
                                .lens(AppData::angle_factor)
                                .padding((5., 5.)),
                                1.,
                            ),
                            0.0,
                        )
                        .with_child(
                            Flex::row().with_child(
                                Button::new("Use golden ratio", |_ctx, data: &mut f64, _: &Env| {
                                    *data = PHI_INV;
                                })
                                .lens(AppData::angle_factor)
                                .padding((5., 5.)),
                                1.,
                            ),
                            0.0,
                        )
                        .padding((10., 3.))
                        .fix_width(200.),
                    0.,
                )
                .with_child(
                    // spinners
                    Flex::column()
                        .with_child(Label::new("Parameters").padding(10.), 0.)
                        .with_child(
                            Flex::row()
                                .with_child(Label::new("Angle factor"), 1.)
                                .with_child(
                                    TextBox::new()
                                        .with_placeholder(DEFAULT_ANGLE_FACTOR.to_string())
                                        .parse()
                                        .lens(AppData::angle_factor.map(
                                            |x| Some(*x),
                                            |x, y| *x = y.unwrap_or(DEFAULT_ANGLE_FACTOR),
                                        )),
                                    2.0,
                                )
                                .with_child(
                                    Stepper::new()
                                        .min(0.)
                                        .max(1.)
                                        .step(0.00001)
                                        .lens(AppData::angle_factor),
                                    0.,
                                )
                                .padding(5.),
                            0.,
                        )
                        .with_child(
                            Flex::row()
                                .with_child(Label::new("Seed number"), 1.)
                                .with_child(
                                    TextBox::new()
                                        .with_placeholder(DEFAULT_N_SEEDS.to_string())
                                        .parse()
                                        .lens(AppData::n_seeds.map(
                                            |x| Some(*x),
                                            |x, y| *x = y.unwrap_or(DEFAULT_N_SEEDS),
                                        )),
                                    2.0,
                                )
                                .with_child(
                                    Stepper::new()
                                        .min(100.)
                                        .max(5000.)
                                        .step(50.)
                                        .lens(AppData::n_seeds),
                                    0.,
                                )
                                .padding(5.),
                            0.,
                        )
                        .with_child(
                            Flex::row()
                                .with_child(Label::new("Radius scale"), 1.)
                                .with_child(
                                    TextBox::new()
                                        .with_placeholder(DEFAULT_RADIUS_SCALE.to_string())
                                        .parse()
                                        .lens(AppData::scale.map(
                                            |x| Some(*x),
                                            |x, y| *x = y.unwrap_or(DEFAULT_RADIUS_SCALE),
                                        )),
                                    2.0,
                                )
                                .with_child(
                                    Stepper::new()
                                        .min(3.)
                                        .max(10.)
                                        .step(0.1)
                                        .lens(AppData::scale),
                                    0.,
                                )
                                .padding(5.),
                            0.,
                        )
                        .padding((10., 3.)),
                    3.,
                )
                .cross_axis_alignment(CrossAxisAlignment::Start),
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
            LocalizedString::new("custom-widget-demo-window-title").with_placeholder("Sunflower"),
        );
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(AppData {
            angle_factor: DEFAULT_ANGLE_FACTOR,
            n_seeds: 600.,
            scale: 4.0,
        })
        .expect("launch failed");
}
