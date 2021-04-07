use druid::widget::{Checkbox, CrossAxisAlignment, Flex, Label, Slider, Stepper, Switch, TextBox};
use druid::{AppLauncher, Data, Lens, LocalizedString, Widget, WidgetExt, WindowDesc};
use piet_common::UnitPoint;

#[derive(Clone, Data, Lens)]
struct AppData {
    option: bool,
    text: String,
    value: f64,

    disabled: bool,
}

fn named_child(name: &str, widget: impl Widget<AppData> + 'static) -> impl Widget<AppData> {
    Flex::row()
        .with_child(Label::new(name))
        .with_default_spacer()
        .with_child(widget)
}

fn main_widget() -> impl Widget<AppData> {
    Flex::column()
        .with_child(named_child("text:", TextBox::new().lens(AppData::text)))
        .with_default_spacer()
        .with_child(named_child(
            "text (disabled):",
            TextBox::new()
                .lens(AppData::text)
                .disabled_if(|data, _| data.disabled),
        ))
        .with_default_spacer()
        .with_child(named_child("text:", TextBox::new().lens(AppData::text)))
        .with_default_spacer()
        .with_child(named_child(
            "text (disabled):",
            TextBox::new()
                .lens(AppData::text)
                .disabled_if(|data, _| data.disabled),
        ))
        .with_default_spacer()
        .with_default_spacer()
        .with_child(named_child(
            "value (disabled):",
            Slider::new()
                .with_range(0.0, 10.0)
                .lens(AppData::value)
                .disabled_if(|data, _| data.disabled),
        ))
        .with_default_spacer()
        .with_child(named_child(
            "value (disabled):",
            Stepper::new()
                .with_range(0.0, 10.0)
                .with_step(0.5)
                .lens(AppData::value)
                .disabled_if(|data, _| data.disabled),
        ))
        .with_default_spacer()
        .with_child(named_child(
            "option (disabled):",
            Checkbox::new("option")
                .lens(AppData::option)
                .disabled_if(|data, _| data.disabled),
        ))
        .with_default_spacer()
        .with_child(named_child(
            "option (disabled):",
            Switch::new()
                .lens(AppData::option)
                .disabled_if(|data, _| data.disabled),
        ))
        .with_default_spacer()
        .with_default_spacer()
        .with_default_spacer()
        .with_child(Checkbox::new("disabled").lens(AppData::disabled))
        .with_default_spacer()
        .cross_axis_alignment(CrossAxisAlignment::End)
        .align_horizontal(UnitPoint::CENTER)
}

pub fn main() {
    let window = WindowDesc::new(main_widget()).title(
        LocalizedString::new("disabled-demo-window-title").with_placeholder("Disabled demo"),
    );
    AppLauncher::with_window(window)
        .log_to_console()
        .launch(AppData {
            option: true,
            text: "a very important text!".to_string(),
            value: 2.0,
            disabled: false,
        })
        .expect("launch failed");
}
