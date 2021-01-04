// Copyright 2020 The Druid Authors.
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

//! This example shows how to draw an image; using images requires
//! enabling the `image` feature in your Cargo.toml; you will also need
//! to specify the image formats you would like to use. See druid/Cargo.toml
//! to see all features.

/* use druid::piet::InterpolationMode;
use druid::widget::prelude::*;
use druid::widget::{FillStrat, Flex, Image, WidgetExt};
use druid::{AppLauncher, Color, ImageBuf, WindowDesc};

pub fn main() {
    let main_window = WindowDesc::new(ui_builder);
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(0)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<u32> {
    let png_data = ImageBuf::from_data(include_bytes!("./assets/PicWithAlpha.png")).unwrap();

    // We create two images, one not having any modifications and the other
    // forced to a fixed width, a fill strategy and an interpolation mode.
    // You can see how this affects the final result. You can play with the
    // interpolation mode to see how this affects things. (Note that this image
    // is already anti-aliased so NearestNeighbor looks weird.)
    Flex::column()
        .with_flex_child(
            Image::new(png_data.clone())
                .fill_mode(FillStrat::FitWidth)
                .interpolation_mode(InterpolationMode::NearestNeighbor)
                .border(Color::WHITE, 1.0)
                .fix_width(150.0)
                .center(),
            1.0,
        )
        .with_flex_child(
            Image::new(png_data)
                .border(Color::WHITE, 1.0),
            1.0,
        )
}
 */

use druid::piet::InterpolationMode;
use druid::widget::{prelude::*, FillStrat, Image};
use druid::widget::{
    Checkbox, CrossAxisAlignment, Flex, Label, RadioGroup, SizedBox, Stepper, TextBox, WidgetExt,
};
use druid::{AppLauncher, Color, Data, ImageBuf, Lens, LensExt, WindowDesc};

const FILL_STRAT_OPTIONS: [(&str, FillStrat); 7] = [
    ("Contain", FillStrat::Contain),
    ("Cover", FillStrat::Cover),
    ("Fill", FillStrat::Fill),
    ("FitHeight", FillStrat::FitHeight),
    ("FitWidth", FillStrat::FitWidth),
    ("None", FillStrat::None),
    ("ScaleDown", FillStrat::ScaleDown),
];

const INTERPOLATION_MODE_OPTIONS: [(&str, InterpolationModeData); 2] = [
    (
        "Bilinear",
        InterpolationModeData::new(InterpolationMode::Bilinear),
    ),
    (
        "NearestNeighbor",
        InterpolationModeData::new(InterpolationMode::NearestNeighbor),
    ),
];

#[derive(Clone, Data, Lens)]
struct AppState {
    demo_state: DemoState,
    params: Params,
}

#[derive(Clone, Data, Lens)]
struct DemoState {
    pub input_text: String,
    pub enabled: bool,
    volume: f64,
}

#[derive(Clone, Data, Lens)]
struct Params {
    debug_layout: bool,
    fill_strat: FillStrat,
    interpolate: bool,
    interpolation_mode: InterpolationModeData,
    fix_width: bool,
    width: f64,
    fix_height: bool,
    height: f64,
}

#[derive(Clone, PartialEq)]
struct InterpolationModeData(InterpolationMode);

impl InterpolationModeData {
    const fn new(mode: InterpolationMode) -> InterpolationModeData {
        InterpolationModeData(mode)
    }
}
impl druid::Data for InterpolationModeData {
    fn same(&self, other: &InterpolationModeData) -> bool {
        self.0 == other.0
    }
}

/// builds a child Flex widget from some paramaters.
struct Rebuilder {
    inner: Box<dyn Widget<AppState>>,
}

impl Rebuilder {
    fn new() -> Rebuilder {
        Rebuilder {
            inner: SizedBox::empty().boxed(),
        }
    }

    fn rebuild_inner(&mut self, data: &AppState) {
        self.inner = build_widget(&data.params);
    }
}

impl Widget<AppState> for Rebuilder {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &AppState, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.rebuild_inner(data);
        }
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &AppState, data: &AppState, env: &Env) {
        if !old_data.params.same(&data.params) {
            self.rebuild_inner(data);
            ctx.children_changed();
        } else {
            self.inner.update(ctx, old_data, data, env);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &AppState,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppState, env: &Env) {
        self.inner.paint(ctx, data, env)
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}

fn make_control_row() -> impl Widget<AppState> {
    Flex::row()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(Label::new("FillStrat:"))
                .with_default_spacer()
                .with_child(RadioGroup::new(FILL_STRAT_OPTIONS.to_vec()).lens(Params::fill_strat)),
        )
        .with_default_spacer()
        .with_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(Label::new("interpolation mode:"))
                .with_default_spacer()
                .with_child(Checkbox::new("set interpolation mode").lens(Params::interpolate))
                .with_default_spacer()
                .with_child(
                    RadioGroup::new(INTERPOLATION_MODE_OPTIONS.to_vec())
                        .lens(Params::interpolation_mode),
                ),
        )
        .with_default_spacer()
        .with_child(make_width())
        .with_default_spacer()
        .with_child(make_height())
        .with_default_spacer()
        .with_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(Label::new("Misc:"))
                .with_default_spacer()
                .with_child(Checkbox::new("Debug layout").lens(Params::debug_layout))
                .with_default_spacer()
                .with_child(Checkbox::new("Fix width").lens(Params::fix_width))
                .with_default_spacer()
                .with_child(Checkbox::new("Fix height").lens(Params::fix_height)),
        )
        .padding(10.0)
        .border(Color::grey(0.6), 2.0)
        .rounded(5.0)
        .lens(AppState::params)
}

fn make_width() -> impl Widget<Params> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("width:"))
        .with_default_spacer()
        .with_child(
            Flex::row()
                .with_child(
                    TextBox::new()
                        .parse()
                        .lens(Params::width.map(|x| Some(*x), |x, y| *x = y.unwrap_or_default()))
                        .fix_width(60.0),
                )
                .with_spacer(druid::theme::WIDGET_CONTROL_COMPONENT_PADDING)
                .with_child(
                    Stepper::new()
                        .with_range(2.0, 50.0)
                        .with_step(2.0)
                        .lens(Params::width),
                ),
        )
}
fn make_height() -> impl Widget<Params> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("height:"))
        .with_default_spacer()
        .with_child(
            Flex::row()
                .with_child(
                    TextBox::new()
                        .parse()
                        .lens(Params::height.map(|x| Some(*x), |x, y| *x = y.unwrap_or_default()))
                        .fix_width(60.0),
                )
                .with_spacer(druid::theme::WIDGET_CONTROL_COMPONENT_PADDING)
                .with_child(
                    Stepper::new()
                        .with_range(2.0, 50.0)
                        .with_step(2.0)
                        .lens(Params::height),
                ),
        )
}

fn build_widget(state: &Params) -> Box<dyn Widget<AppState>> {
    let png_data = ImageBuf::from_data(include_bytes!("./assets/PicWithAlpha.png")).unwrap();

    let mut img = Image::new(png_data.clone()).fill_mode(state.fill_strat);
    if state.interpolate {
        img.set_interpolation_mode(state.interpolation_mode.0)
    }
    let mut sized = SizedBox::new(img);
    if state.fix_width {
        sized = sized.fix_width(state.width)
    }
    if state.fix_height {
        sized = sized.fix_height(state.height)
    }
    if state.debug_layout {
        sized.center().debug_paint_layout().boxed()
    } else {
        sized.center().boxed()
    }
}

fn make_ui() -> impl Widget<AppState> {
    Flex::column()
        .must_fill_main_axis(true)
        .with_child(make_control_row())
        .with_default_spacer()
        .with_flex_child(Rebuilder::new().center(), 1.0)
        .padding(10.0)
}

pub fn main() {
    let main_window = WindowDesc::new(make_ui)
        .window_size((720., 600.))
        .with_min_size((620., 300.))
        .title("Flex Container Options");

    let demo_state = DemoState {
        input_text: "hello".into(),
        enabled: false,
        volume: 0.0,
    };

    let params = Params {
        debug_layout: false,
        fill_strat: FillStrat::None,
        interpolate: false,
        interpolation_mode: InterpolationModeData::new(InterpolationMode::Bilinear),
        fix_width: false,
        width: 0.0,
        fix_height: false,
        height: 0.0,
    };

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(AppState { demo_state, params })
        .expect("Failed to launch application");
}
