# Widgets overview

The `Widget` trait represents components of your UI. Druid includes a set of
built-in widgets, and you can also write your own. You combine the built-in
and custom widgets to create a *widget tree*; you will start with some single
*root widget*, which will (generally) have children, which may themselves have
children, and so on. `Widget` has a generic paramater `T` that represents
the [`Data`] handled by that widget. Some widgets (such as layout widgets)
may be entirely agnostic about what sort of `Data` they encounter, while other
widgets (such as a slider) may expect a single type (such as `f64`).

> **Note**: For more information on how different parts of your [`Data`] are exposed
to different widgets, see [`Lens`].

At a high level, druid works like this:

- **event**: an `Event` arrives from the operating system, such as a key press,
a mouse movement, or a timer firing. This event is delivered to your root
widget's [`event`] method. This method is provided **mutable** access to your
application model; this is the only place where your model can change. Depending
on the type of `Event` and the implementation of your [`event`] method, this
event is then delivered recursively down the tree until it is handled.
- **update**: After this call returns, the framework checks to see if the data was mutated.
  If so, it calls your root widget's [`update`] method, passing in both the new
  data as well as the previous data. Your widget can then update any internal
  state (if required) or can request a [`layout`] or a [`paint`] call if the
  its appearance is no longer valid.
- After [`update`] returns, the framework checks to see if any widgets in a
  given window have indicated that they need layout or paint. If so, the
  framework will call the following methods:
- **layout**: This is where the framework determines where to position each
  widget on the screen. Druid uses a layout system heavily inspired by Flutter's
  [box layout model]: widgets are passed constraints, in the form of a minimum
  and a maximum allowed size, and they return a size in that range.
- **paint**: After `layout`, the framework calls your widget's `paint` method.
This is where your widget draws itself, using a familiar imperative 2D graphics
API.
- In addition to these four methods, there is also **lifecycle**, which is
  called in response to various changes to framework state; it is not called
  predictably during event handling, but only when extra information (such
  as if a widget has gained focuse) happens as a consequence of other events.

For more information on implementing these methods, see [Creating custom
widgets].

## Modularity and composition

Widgets are intended to be modular and composable, not monolithic. For instance,
widgets generally do not control their own alignment or padding; if you have
a button, and you would like it to have 8px of horizontal padding and 4px of
vertical padding, you can just do,

```rust
use druid::widget::{Button, Padding};

fn padded_button<T>() -> impl Widget<T> {
    let button = Button::new("Humour me", Button::NOOP);
    let padded = Padding::new(button, (4.0, 8.0));
    padded
}
```
to force the button to be aligned center-left if it is given extra space you can
write,

```rust
use druid::widget::Align;

fn align_left<T>(widget: impl Widget<T>) -> impl Widget<T> {
    Align::left(widget)
}
```

## Builder methods and `WidgetExt`

Widgets are generally constructed using builder-style methods. Unlike the normal
[builder pattern], we generally do not separate the type that is
built from the builder type; instead the builder methods are on the widget
itself.

```rust
use druid::widget::Stepper;

fn main() {
    // a Stepper with defualt paramaters
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
```

Additionally, there are a large number of helper methods available on all
widgets, as part of the `WidgetExt` trait. These builder-style methods take one
widget and wrap it in another:

```rust
use druid::widget::{Stepper, WidgetExt};

fn main() {
    let padded_and_center_aligned_stepper = Stepper::new()
        .with_range(10.0, 50.0)
        .padding(8.0)
        .center();
}
```

These builder-style methods also exist on containers. For instance, to create
a stack of three labels, you can do:

```rust
use druid::widget::{Label, Flex};

fn main() {
    let vstack = Flex::column()
        .with_child(Label::new("Number One"))
        .with_child(Label::new("Number Two"))
        .with_child(Label::new("Some Other Number"));
}
```

[`event`]: #event
[`lifecycle`]: #lifecycle
[`update`]: #update
[`layout`]: #layout
[`paint`]: #paint
[`Data`]: ./data.md
[`Lens`]: ./lens.md
[box layout model]: https://api.flutter.dev/flutter/rendering/BoxConstraints-class.html
[Creating custom widgets]: ./custom_widgets.md
[builder pattern]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
