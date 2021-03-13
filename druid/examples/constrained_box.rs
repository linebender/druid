use druid::{
    widget::{ConstrainedBox, Flex, Label, LineBreaking, MainAxisAlignment},
    AppLauncher, BoxConstraints, Color, Data, Env, Lens, Size, Widget, WidgetExt, WindowDesc,
};

fn main() {
    let window = WindowDesc::new(ui());
    let too_small_text=
        "This is a box that forces it's child to be within the min and max constraints given to it. Notice the label text has a fixed width and height of 50.0 but is forced to be 150.0 and 300.0 which are the min width and height provided to the ConstrainedBox.";
    let too_large_text =
        "This is a box that forces it's child to be within the min and max constraints given to it. Notice the label text has a fixed width and height of 450.0 but is forced to be 250.0 which is the max width and height provided to the ConstrainedBox.";
    AppLauncher::with_window(window)
        .launch(AppState {
            too_small_text: too_small_text.to_string(),
            too_large_text: too_large_text.to_string(),
        })
        .unwrap();
}
#[derive(Clone, Data, Lens, Debug)]
struct AppState {
    too_small_text: String,
    too_large_text: String,
}

fn ui() -> impl Widget<AppState> {
    let too_small_label = Label::new(|data: &String, _env: &Env| data.clone())
        .with_text_color(Color::WHITE)
        .with_line_break_mode(LineBreaking::WordWrap)
        .center()
        .lens(AppState::too_small_text)
        .fix_width(50.)
        .height(50.);

    let bc = BoxConstraints::new(Size::new(150., 300.), Size::new(250., 650.));
    let too_small_box = ConstrainedBox::new(too_small_label, bc).background(Color::GRAY);

    let too_large_label = Label::new(|data: &String, _env: &Env| data.clone())
        .with_text_color(Color::BLACK)
        .with_line_break_mode(LineBreaking::WordWrap)
        .center()
        .lens(AppState::too_large_text)
        .fix_width(450.)
        .height(450.);

    let bc = BoxConstraints::new(Size::new(100., 125.), Size::new(250., 250.));
    let too_large_box = ConstrainedBox::new(too_large_label, bc).background(Color::WHITE);

    Flex::column()
        .with_child(too_small_box)
        .with_child(too_large_box)
        .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
}
