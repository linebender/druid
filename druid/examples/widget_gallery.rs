// Copyright 2020 The xi-editor Authors.
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

use druid::{
    im,
    kurbo::{Affine, BezPath, Circle, Point},
    piet::{FixedLinearGradient, GradientStop, InterpolationMode},
    widget::{
        prelude::*, Button, Checkbox, FillStrat, Flex, Image, Label, List, Painter, ProgressBar,
        RadioGroup, Scroll, Slider, Spinner, Stepper, Switch, TextBox,
    },
    AppLauncher, Color, Data, ImageBuf, Lens, Widget, WidgetExt, WidgetPod, WindowDesc,
};

#[cfg(not(target_arch = "wasm32"))]
use druid::widget::{Svg, SvgData};

const XI_IMAGE: &[u8] = include_bytes!("assets/xi.image");

#[derive(Clone, Data, Lens)]
struct AppData {
    label_data: String,
    checkbox_data: bool,
    clicked_count: u64,
    list_items: im::Vector<String>,
    progressbar: f64,
    radio: MyRadio,
    stepper: f64,
    editable_text: String,
}

#[derive(Clone, Data, PartialEq)]
enum MyRadio {
    GaGa,
    GuGu,
    BaaBaa,
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder).title("Widget Gallery");
    // Set our initial data
    let data = AppData {
        label_data: "test".into(),
        checkbox_data: false,
        clicked_count: 0,
        list_items: im::vector!["1".into(), "2".into(), "3".into()],
        progressbar: 0.5,
        radio: MyRadio::GaGa,
        stepper: 0.0,
        editable_text: "edit me!".into(),
    };
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<AppData> {
    #[cfg(not(target_arch = "wasm32"))]
    let svg_example = label_widget(
        Svg::new(
            include_str!("./assets/tiger.svg")
                .parse::<SvgData>()
                .unwrap(),
        ),
        "Svg",
    );

    #[cfg(target_arch = "wasm32")]
    let svg_example = label_widget(Label::new("no SVG on wasm (yet)").center(), "Svg");

    Scroll::new(
        SquaresGrid::new()
            .with_cell_size(Size::new(200.0, 240.0))
            .with_spacing(20.0)
            .with_child(label_widget(
                Label::new(|data: &AppData, _: &_| data.label_data.clone()),
                "Label",
            ))
            .with_child(label_widget(
                Flex::column()
                    .with_child(
                        Button::new("Click me!")
                            .on_click(|_, data: &mut AppData, _: &_| data.clicked_count += 1),
                    )
                    .with_spacer(4.0)
                    .with_child(Label::new(|data: &AppData, _: &_| {
                        format!("Clicked {} times!", data.clicked_count)
                    })),
                "Button",
            ))
            .with_child(label_widget(
                Checkbox::new("Check me!").lens(AppData::checkbox_data),
                "Checkbox",
            ))
            .with_child(label_widget(
                List::new(|| {
                    Label::new(|data: &String, _: &_| format!("List item: {}", data))
                        .center()
                        .background(Color::hlc(230.0, 50.0, 50.0))
                        .fix_height(40.0)
                        .expand_width()
                })
                .lens(AppData::list_items),
                "List",
            ))
            .with_child(label_widget(
                Flex::column()
                    .with_child(ProgressBar::new().lens(AppData::progressbar))
                    .with_spacer(4.0)
                    .with_child(Label::new(|data: &AppData, _: &_| {
                        format!("{:.1}%", data.progressbar * 100.0)
                    }))
                    .with_spacer(4.0)
                    .with_child(
                        Flex::row()
                            .with_child(Button::new("<<").on_click(|_, data: &mut AppData, _| {
                                data.progressbar = (data.progressbar - 0.05).max(0.0);
                            }))
                            .with_spacer(4.0)
                            .with_child(Button::new(">>").on_click(|_, data: &mut AppData, _| {
                                data.progressbar = (data.progressbar + 0.05).min(1.0);
                            })),
                    ),
                "ProgressBar",
            ))
            // The image example here uses hard-coded literal image data included in the binary.
            // You may also want to load an image at runtime using a crate like `image`.
            .with_child(label_widget(
                Painter::new(paint_example).fix_size(32.0, 32.0),
                "Painter",
            ))
            .with_child(label_widget(
                RadioGroup::new(vec![
                    ("radio gaga", MyRadio::GaGa),
                    ("radio gugu", MyRadio::GuGu),
                    ("radio baabaa", MyRadio::BaaBaa),
                ])
                .lens(AppData::radio),
                "RadioGroup",
            ))
            .with_child(label_widget(
                Flex::column()
                    .with_child(Slider::new().lens(AppData::progressbar))
                    .with_spacer(4.0)
                    .with_child(Label::new(|data: &AppData, _: &_| {
                        format!("{:3.0}%", data.progressbar * 100.0)
                    })),
                "Slider",
            ))
            .with_child(label_widget(
                Flex::row()
                    .with_child(Stepper::new().lens(AppData::stepper))
                    .with_spacer(4.0)
                    .with_child(Label::new(|data: &AppData, _: &_| {
                        format!("{:.1}", data.stepper)
                    })),
                "Stepper",
            ))
            .with_child(label_widget(
                TextBox::new().lens(AppData::editable_text),
                "TextBox",
            ))
            .with_child(label_widget(
                Switch::new().lens(AppData::checkbox_data),
                "Switch",
            ))
            .with_child(label_widget(
                Spinner::new().fix_height(40.0).center(),
                "Spinner",
            ))
            .with_child(label_widget(
                Image::new(
                    ImageBuf::from_data(include_bytes!("./assets/PicWithAlpha.png")).unwrap(),
                )
                .fill_mode(FillStrat::Fill)
                .interpolation_mode(InterpolationMode::Bilinear),
                "Image",
            ))
            .with_child(svg_example),
    )
    .vertical()
}

fn label_widget<T: Data>(widget: impl Widget<T> + 'static, label: &str) -> impl Widget<T> {
    Flex::column()
        .must_fill_main_axis(true)
        .with_flex_child(widget.center(), 1.0)
        .with_child(
            Painter::new(|ctx, _: &_, _: &_| {
                let size = ctx.size().to_rect();
                ctx.fill(size, &Color::WHITE)
            })
            .fix_height(1.0),
        )
        .with_child(Label::new(label).center().fix_height(40.0))
        .border(Color::WHITE, 1.0)
}

fn load_xi_image<Ctx: druid::RenderContext>(ctx: &mut Ctx) -> Ctx::Image {
    ctx.make_image(32, 32, XI_IMAGE, druid::piet::ImageFormat::Rgb)
        .unwrap()
}

fn paint_example<T>(ctx: &mut PaintCtx, _: &T, _env: &Env) {
    let bounds = ctx.size().to_rect();
    let img = load_xi_image(ctx.render_ctx);
    ctx.draw_image(
        &img,
        bounds,
        druid::piet::InterpolationMode::NearestNeighbor,
    );
    ctx.with_save(|ctx| {
        ctx.transform(Affine::scale_non_uniform(bounds.width(), bounds.height()));
        // Draw the dot of the `i` on top of the image data.
        let i_dot = Circle::new((0.775, 0.18), 0.05);
        let i_dot_brush = ctx.solid_brush(Color::WHITE);
        ctx.fill(i_dot, &i_dot_brush);
        // Cross out Xi because it's going dormant :'(
        let mut spare = BezPath::new();
        spare.move_to((0.1, 0.1));
        spare.line_to((0.2, 0.1));
        spare.line_to((0.9, 0.9));
        spare.line_to((0.8, 0.9));
        spare.close_path();
        let spare_brush = ctx
            .gradient(FixedLinearGradient {
                start: (0.0, 0.0).into(),
                end: (1.0, 1.0).into(),
                stops: vec![
                    GradientStop {
                        pos: 0.0,
                        color: Color::rgb(1.0, 0.0, 0.0),
                    },
                    GradientStop {
                        pos: 1.0,
                        color: Color::rgb(0.4, 0.0, 0.0),
                    },
                ],
            })
            .unwrap();
        ctx.fill(spare, &spare_brush);
    });
}

// Grid widget

const DEFAULT_GRID_CELL_SIZE: Size = Size::new(100.0, 100.0);
const DEFAULT_GRID_SPACING: f64 = 10.0;

pub struct SquaresGrid<T> {
    widgets: Vec<WidgetPod<T, Box<dyn Widget<T>>>>,
    /// The number of widgets we can fit in the grid given the grid size.
    drawable_widgets: usize,
    cell_size: Size,
    spacing: f64,
}

impl<T> SquaresGrid<T> {
    pub fn new() -> Self {
        SquaresGrid {
            widgets: vec![],
            drawable_widgets: 0,
            cell_size: DEFAULT_GRID_CELL_SIZE,
            spacing: DEFAULT_GRID_SPACING,
        }
    }

    pub fn with_spacing(mut self, spacing: f64) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn with_cell_size(mut self, cell_size: Size) -> Self {
        self.cell_size = cell_size;
        self
    }

    pub fn with_child(mut self, widget: impl Widget<T> + 'static) -> Self {
        self.widgets.push(WidgetPod::new(Box::new(widget)));
        self
    }
}

impl<T> Default for SquaresGrid<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Data> Widget<T> for SquaresGrid<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        for widget in self.widgets.iter_mut() {
            widget.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        for widget in self.widgets.iter_mut() {
            widget.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        for widget in self.widgets.iter_mut() {
            widget.update(ctx, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let count = self.widgets.len() as f64;
        // The space needed to lay all elements out on a single line.
        let ideal_width = (self.cell_size.width + self.spacing + 1.0) * count;
        // Constrain the width.
        let width = ideal_width.min(bc.max().width).max(bc.min().width);
        // Given the width, the space needed to lay out all elements (as many as possible on each
        // line).
        let cells_in_row =
            ((width - self.spacing) / (self.cell_size.width + self.spacing)).floor() as usize;
        let (height, rows) = if cells_in_row > 0 {
            let mut rows = (count / cells_in_row as f64).ceil() as usize;
            let height_from_rows =
                |rows: usize| (rows as f64) * (self.cell_size.height + self.spacing) + self.spacing;
            let ideal_height = height_from_rows(rows);

            // Constrain the height
            let height = ideal_height.max(bc.min().height).min(bc.max().height);
            // Now calcuate how many rows we can actually fit in
            while height_from_rows(rows) > height && rows > 0 {
                rows -= 1;
            }
            (height, rows)
        } else {
            (bc.min().height, 0)
        };
        // Constrain the number of drawn widgets by the number there is space to draw.
        self.drawable_widgets = self.widgets.len().min(rows * cells_in_row);
        // Now we have the width and height, we can lay out the children.
        let mut x_position = self.spacing;
        let mut y_position = self.spacing;
        for (idx, widget) in self
            .widgets
            .iter_mut()
            .take(self.drawable_widgets)
            .enumerate()
        {
            widget.layout(
                ctx,
                &BoxConstraints::new(self.cell_size, self.cell_size),
                data,
                env,
            );
            widget.set_origin(ctx, data, env, Point::new(x_position, y_position));
            // Increment position for the next cell
            x_position += self.cell_size.width + self.spacing;
            // If we can't fit in another cell in this row ...
            if (idx + 1) % cells_in_row == 0 {
                // ... then start a new row.
                x_position = self.spacing;
                y_position += self.cell_size.height + self.spacing;
            }
        }
        Size { width, height }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        for widget in self.widgets.iter_mut().take(self.drawable_widgets) {
            widget.paint(ctx, data, env);
        }
    }
}
