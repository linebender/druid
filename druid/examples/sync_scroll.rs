use druid::widget::prelude::*;
use druid::widget::{Flex, Label, Padding, Scroll, ScrollTo, TextBox, SCROLL_TO};
use druid::{AppLauncher, LocalizedString, WidgetExt, WindowDesc};

pub fn main() {
    let window = WindowDesc::new(build_widget)
        .title(LocalizedString::new("scroll-demo-window-title").with_placeholder("Scroll demo"));
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch("Synchronised scrolling".into())
        .expect("launch failed");
}

fn build_widget() -> impl Widget<String> {
    let mut row = Flex::row();
    let follower_id = WidgetId::next();

    let mut leader = Scroll::new(make_col(0));

    leader.add_scroll_handler(move |ctx, scroll_offsets| {
        ctx.submit_command(SCROLL_TO.with(ScrollTo::y(scroll_offsets.y)), follower_id);
    });

    row.add_child(leader);

    row.add_child(Scroll::new(make_col(1)).with_id(follower_id));

    row
}

fn make_col(i: i32) -> Flex<String> {
    let mut col = Flex::column();

    for j in 0..100 {
        if i == j {
            col.add_child(Padding::new(3.0, TextBox::new()));
        } else {
            col.add_child(Padding::new(
                3.0,
                Label::new(move |d: &String, _env: &_| format!("Label {}, {}, {}", i, j, d)),
            ));
        };
    }
    col
}
