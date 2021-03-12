use druid::{
    widget::{AspectRatioBox, Flex, Label, LineBreaking, MainAxisAlignment, SizedBox},
    AppLauncher, Color, Data, Env, Lens, Widget, WidgetExt, WindowDesc,
};

fn main() {
    let window = WindowDesc::new(ui());
    let fixed_message =
        "Hello there, this is a fixed size box and it will not change no matter what.";
    let aspect_ratio_message =
        "Hello there, this is a box that maintains it's aspect-ratio as best as possible. Notice text will overflow if box becomes too small.";
    AppLauncher::with_window(window)
        .use_env_tracing()
        .launch(AppState {
            fixed_box: fixed_message.to_string(),
            aspect_ratio_box: aspect_ratio_message.to_string(),
        })
        .unwrap();
}
#[derive(Clone, Data, Lens, Debug)]
struct AppState {
    fixed_box: String,
    aspect_ratio_box: String,
}

fn ui() -> impl Widget<AppState> {
    let fixed_label = Label::new(|data: &String, _env: &Env| data.clone())
        .with_text_color(Color::BLACK)
        .with_line_break_mode(LineBreaking::WordWrap)
        .center()
        .lens(AppState::fixed_box);
    let fixed_box = SizedBox::new(fixed_label)
        .height(250.)
        .width(250.)
        .background(Color::WHITE);

    let aspect_ratio_label = Label::new(|data: &String, _env: &Env| data.clone())
        .with_text_color(Color::BLACK)
        .with_line_break_mode(LineBreaking::WordWrap)
        .center()
        .lens(AppState::aspect_ratio_box);
    let aspect_ratio_box = AspectRatioBox::new(aspect_ratio_label, 2.0)
        .border(Color::BLACK, 1.0)
        .background(Color::WHITE);

    Flex::column()
        .with_flex_child(fixed_box, 1.0)
        // using flex child so that aspect_ratio doesn't get any infinite constraints
        // you can use this in `with_child` but there might be some unintended behavior
        // the aspect ratio box will work correctly but it might use up all the
        // allotted space and make any flex children disappear
        .with_flex_child(aspect_ratio_box, 1.0)
        .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
}
