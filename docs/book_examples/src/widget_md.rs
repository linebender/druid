// ANCHOR: padded_label
use druid::widget::{Label, Padding};

fn padded_label() {
    let label: Label<()> = Label::new("Humour me");
    let padded = Padding::new((4.0, 8.0), label);
}
// ANCHOR_END: padded_label

// ANCHOR: align_center
use druid::widget::Align;

fn align_center() {
    let label: Label<()> = Label::new("Center me");
    let centered = Align::centered(label);
}
// ANCHOR_END: align_center

// ANCHOR: stepper_builder
use druid::widget::Stepper;

fn steppers() {
    // A Stepper with default parameters
    let stepper1 = Stepper::new();

    // A Stepper that operates over a custom range
    let stepper2 = Stepper::new().with_range(10.0, 50.0);

    // A Stepper with a custom range *and* a custom step size, that
    // wraps around past its min and max values:
    let stepper3 = Stepper::new()
        .with_range(10.0, 50.0)
        .with_step(2.5)
        .with_wraparound(true);
}
// ANCHOR_END: stepper_builder

#[rustfmt::skip]
mod padded_stepper_raw {
// ANCHOR: padded_stepper_raw
use druid::widget::{Align, Padding, Stepper};

fn padded_stepper() {
    let stepper = Stepper::new().with_range(10.0, 50.0);
    let padding = Padding::new(8.0, stepper);
    let padded_and_center_aligned_stepper = Align::centered(padding);
}
// ANCHOR_END: padded_stepper_raw
}

#[rustfmt::skip]
mod padded_stepper_widgetext {
// ANCHOR: padded_stepper_widgetext
use druid::widget::{Stepper, WidgetExt};

fn padded_stepper() {
    let padded_and_center_aligned_stepper =
        Stepper::new().with_range(10.0, 50.0).padding(8.0).center();
}
// ANCHOR_END: padded_stepper_widgetext
}

// ANCHOR: flex_builder
use druid::widget::Flex;

fn flex_builder() -> Flex<()> {
    Flex::column()
        .with_child(Label::new("Number One"))
        .with_child(Label::new("Number Two"))
        .with_child(Label::new("Some Other Number"))
}
// ANCHOR_END: flex_builder
