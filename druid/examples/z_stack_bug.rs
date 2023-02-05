use druid::widget::{CrossAxisAlignment, Flex, Label, Scroll, SizedBox, ZStack};
use druid::{AppLauncher, Color, UnitPoint, Widget, WidgetExt, WindowDesc};

fn main() {
    let window = WindowDesc::new(build_ui());

    AppLauncher::with_window(window)
        .log_to_console()
        .launch(())
        .unwrap();
}

fn build_ui() -> impl Widget<()> {
    let mut container = Flex::column().cross_axis_alignment(CrossAxisAlignment::Fill);

    for _ in 0..10 {
        let stack = ZStack::new(
            Label::new("Base layer")
                .align_vertical(UnitPoint::TOP)
                .expand_width()
                .fix_height(200.0)
                .background(Color::grey8(20)),
        )
        .with_centered_child(
            Label::new("Overlay")
                .center()
                .fix_height(100.0)
                .background(Color::grey8(0)),
        );

        container.add_child(SizedBox::empty().height(200.0));
        container.add_child(
            Flex::row()
                .with_flex_child(stack, 1.0)
                .with_default_spacer()
                .with_child(SizedBox::empty()),
        );
    }

    Scroll::new(container).vertical()
}
