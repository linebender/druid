use druid::{Widget, WindowDesc, LocalizedString, AppLauncher, WidgetExt, lens};
use druid::widget::{RangeSlider, Flex, CrossAxisAlignment, SliderAnnotation, ViewSwitcher, Slider};

pub fn build_widget() -> impl Widget<((f64, f64), f64)> {
    Flex::column()
        .with_child(
            SliderAnnotation::new(
                RangeSlider::new()
                    .with_range(0.0, 6.0)
                    .with_min_distance(0.1)
                    .snap(0.1)
                    .view_track(),
                0.5,
                4
            ).lens(lens!(((f64, f64), f64), 0))
        )
        .with_default_spacer()
        .with_child(
            ViewSwitcher::new(
                |data: &((f64, f64), f64), _| data.0,
                |data, _, _|Box::new(
                    SliderAnnotation::new(
                        Slider::new()
                            .with_range(data.0, data.1)
                            .snap(0.5)
                            .view_track(),
                        0.5,
                        0
                    ).lens(lens!(((f64, f64), f64), 1))
                ) as Box<dyn Widget<((f64, f64), f64)>>
            )
        )
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .padding(20.0)
}

pub fn main() {
    let window = WindowDesc::new(build_widget())
        .title(LocalizedString::new("scroll-demo-window-title").with_placeholder("Slider demo"));
    AppLauncher::with_window(window)
        .use_env_tracing()
        .launch(((1.0, 5.0), 3.0))
        .expect("launch failed");
}