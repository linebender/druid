use druid::widget::Label;
use druid::{Color, Key, WidgetExt};

// ANCHOR: key_or_value
const IMPORTANT_LABEL_COLOR: Key<Color> = Key::new("org.linebender.example.important-label-color");
const RED: Color = Color::rgb8(0xFF, 0, 0);

fn make_labels() {
    let with_value = Label::<()>::new("Warning!").with_text_color(RED);
    let with_key = Label::<()>::new("Warning!").with_text_color(IMPORTANT_LABEL_COLOR);
}
// ANCHOR_END: key_or_value

// ANCHOR: env_scope
fn scoped_label() {
    let my_label = Label::<()>::new("Warning!").env_scope(|env, _| {
        env.set(druid::theme::LABEL_COLOR, Color::BLACK);
        env.set(druid::theme::TEXT_SIZE_NORMAL, 18.0);
    });
}
// ANCHOR_END: env_scope
