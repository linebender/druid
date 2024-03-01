// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This showcase demonstrates how to use the image widget and its
//! properties. You can change the parameters in the GUI to see how
//! everything behaves.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::piet::InterpolationMode;
use druid::text::ParseFormatter;
use druid::widget::{prelude::*, FillStrat, Image};
use druid::widget::{
    Checkbox, CrossAxisAlignment, Flex, Label, RadioGroup, SizedBox, TextBox, WidgetExt,
};
use druid::{AppLauncher, Color, Data, ImageBuf, Lens, Rect, WindowDesc};

static FILL_STRAT_OPTIONS: &[(&str, FillStrat)] = &[
    ("Contain", FillStrat::Contain),
    ("Cover", FillStrat::Cover),
    ("Fill", FillStrat::Fill),
    ("FitHeight", FillStrat::FitHeight),
    ("FitWidth", FillStrat::FitWidth),
    ("None", FillStrat::None),
    ("ScaleDown", FillStrat::ScaleDown),
];

static INTERPOLATION_MODE_OPTIONS: &[(&str, InterpolationMode)] = &[
    ("Bilinear", InterpolationMode::Bilinear),
    ("NearestNeighbor", InterpolationMode::NearestNeighbor),
];
#[derive(Clone, Data, Lens)]
struct AppState {
    fill_strat: FillStrat,
    interpolate: bool,
    interpolation_mode: InterpolationMode,
    fix_width: bool,
    width: f64,
    fix_height: bool,
    height: f64,
    clip: bool,
    clip_x: f64,
    clip_y: f64,
    clip_width: f64,
    clip_height: f64,
}

/// builds a child Flex widget from some parameters.
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
        self.inner = build_widget(data);
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

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &AppState, data: &AppState, _env: &Env) {
        if !old_data.same(data) {
            self.rebuild_inner(data);
            ctx.children_changed();
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
                .with_child(
                    RadioGroup::column(FILL_STRAT_OPTIONS.to_vec()).lens(AppState::fill_strat),
                ),
        )
        .with_default_spacer()
        .with_child(
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(Label::new("interpolation mode:"))
                .with_default_spacer()
                .with_child(
                    RadioGroup::column(INTERPOLATION_MODE_OPTIONS.to_vec())
                        .lens(AppState::interpolation_mode),
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
                .with_child(Checkbox::new("Fix width").lens(AppState::fix_width))
                .with_default_spacer()
                .with_child(Checkbox::new("Fix height").lens(AppState::fix_height))
                .with_default_spacer()
                .with_child(Checkbox::new("Clip").lens(AppState::clip))
                .with_default_spacer()
                .with_child(Checkbox::new("set interpolation mode").lens(AppState::interpolate)),
        )
        .padding(10.0)
        .border(Color::grey(0.6), 2.0)
        .rounded(5.0)
}

fn make_width() -> impl Widget<AppState> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("width:"))
        .with_default_spacer()
        .with_child(
            Flex::row().with_child(
                TextBox::new()
                    .with_formatter(ParseFormatter::new())
                    .lens(AppState::width)
                    .fix_width(60.0),
            ),
        )
        .with_default_spacer()
        .with_child(Label::new("clip x:"))
        .with_default_spacer()
        .with_child(
            Flex::row().with_child(
                TextBox::new()
                    .with_formatter(ParseFormatter::new())
                    .lens(AppState::clip_x)
                    .fix_width(60.0),
            ),
        )
        .with_default_spacer()
        .with_child(Label::new("clip width:"))
        .with_default_spacer()
        .with_child(
            Flex::row().with_child(
                TextBox::new()
                    .with_formatter(ParseFormatter::new())
                    .lens(AppState::clip_width)
                    .fix_width(60.0),
            ),
        )
}
fn make_height() -> impl Widget<AppState> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new("height:"))
        .with_default_spacer()
        .with_child(
            Flex::row().with_child(
                TextBox::new()
                    .with_formatter(ParseFormatter::new())
                    .lens(AppState::height)
                    .fix_width(60.0),
            ),
        )
        .with_default_spacer()
        .with_child(Label::new("clip y:"))
        .with_default_spacer()
        .with_child(
            Flex::row().with_child(
                TextBox::new()
                    .with_formatter(ParseFormatter::new())
                    .lens(AppState::clip_y)
                    .fix_width(60.0),
            ),
        )
        .with_default_spacer()
        .with_child(Label::new("clip height:"))
        .with_default_spacer()
        .with_child(
            Flex::row().with_child(
                TextBox::new()
                    .with_formatter(ParseFormatter::new())
                    .lens(AppState::clip_height)
                    .fix_width(60.0),
            ),
        )
}

fn build_widget(state: &AppState) -> Box<dyn Widget<AppState>> {
    let png_data = ImageBuf::from_data(include_bytes!("./assets/PicWithAlpha.png")).unwrap();

    let mut img = Image::new(png_data).fill_mode(state.fill_strat);
    if state.interpolate {
        img.set_interpolation_mode(state.interpolation_mode)
    }
    if state.clip {
        img.set_clip_area(Some(Rect::new(
            state.clip_x,
            state.clip_y,
            state.clip_x + state.clip_width,
            state.clip_y + state.clip_height,
        )));
    }
    let mut sized = SizedBox::new(img);
    if state.fix_width {
        sized = sized.fix_width(state.width)
    }
    if state.fix_height {
        sized = sized.fix_height(state.height)
    }
    sized.border(Color::grey(0.6), 2.0).center().boxed()
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
    let main_window = WindowDesc::new(make_ui())
        .window_size((650., 450.))
        .title("Flex Container Options");

    let state = AppState {
        fill_strat: FillStrat::Cover,
        interpolate: true,
        interpolation_mode: InterpolationMode::Bilinear,
        fix_width: true,
        width: 200.,
        fix_height: true,
        height: 100.,
        clip: false,
        clip_x: 0.,
        clip_y: 0.,
        clip_width: 50.,
        clip_height: 50.,
    };

    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(state)
        .expect("Failed to launch application");
}
