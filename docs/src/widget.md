# Widgets and the `Widget` trait

The `Widget` trait represents components of your UI. Druid includes a set of
built-in widgets, and you can also write your own. You combine the built-in
and custom widgets to create a *widget tree*; you will start with some single
*root widget*, which will (generally) have children, which may themselves have
children, and so on. `Widget` has a generic parameter `T` that represents
the [`Data`] handled by that widget. Some widgets (such as layout widgets)
may be entirely agnostic about what sort of `Data` they encounter, while other
widgets (such as a slider) may expect a single type (such as `f64`).

> **Note**: For more information on how different parts of your [`Data`] are exposed
to different widgets, see [`Lens`].

At a high level, Druid works like this:

- **event**: an `Event` arrives from the operating system, such as a key press,
a mouse movement, or a timer firing. This event is delivered to your root
widget's `event` method. This method is provided **mutable** access to your
application model; this is the only place where your model can change. Depending
on the type of `Event` and the implementation of your `event` method, this
event is then delivered recursively down the tree until it is handled.
- **update**: After this call returns, the framework checks to see if the data was mutated.
  If so, it calls your root widget's `update` method, passing in both the new
  data as well as the previous data. Your widget can then update any internal
  state (data that the widget uses that is not part of the application model,
  such as appearance data) and can request a `layout` or a `paint` call if
  its appearance is no longer valid.
- After `update` returns, the framework checks to see if any widgets in a
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
  as if a widget has gained focus) happens as a consequence of other events.

For more information on implementing these methods, see [Creating custom
widgets].

## Modularity and composition

Widgets are intended to be modular and composable, not monolithic. For instance,
widgets generally do not control their own alignment or padding; if you have
a label, and you would like it to have 8dp of horizontal padding and 4dp of
vertical padding, you can just do,

```rust,noplaypen
{{#include ../book_examples/src/widget_md.rs:padded_label}}
```
to force the label to be center-aligned if it is given extra space you can write,

```rust,noplaypen
{{#include ../book_examples/src/widget_md.rs:align_center}}
```

## Builder methods and `WidgetExt`

Widgets are generally constructed using builder-style methods. Unlike the normal
[builder pattern], we generally do not separate the type that is
built from the builder type; instead the builder methods are on the widget
itself.

```rust,noplaypen
{{#include ../book_examples/src/widget_md.rs:stepper_builder}}
```

Additionally, there are a large number of helper methods available on all
widgets, as part of the `WidgetExt` trait. These builder-style methods take one
widget and wrap it in another. The following two functions produce the same
output:

**Explicit**:
```rust,noplaypen
{{#include ../book_examples/src/widget_md.rs:padded_stepper_raw}}
```

**WidgetExt**:
```rust,noplaypen
{{#include ../book_examples/src/widget_md.rs:padded_stepper_widgetext}}
```

These builder-style methods also exist on containers. For instance, to create
a stack of three labels, you can do:

```rust,noplaypen
{{#include ../book_examples/src/widget_md.rs:flex_builder}}
```

[`Data`]: ./data.md
[`Lens`]: ./lens.md
[box layout model]: https://api.flutter.dev/flutter/rendering/BoxConstraints-class.html
[Creating custom widgets]: ./custom_widgets.md
[builder pattern]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
