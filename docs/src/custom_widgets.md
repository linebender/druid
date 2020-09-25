# Create custom widgets

The `Widget` trait is the heart of Druid, and in any serious application you
will eventually need to create and use custom `Widget`s.

## `Painter` and `Controller`

There are two helper widgets in Druid that let you customize widget behaviour
without needing to implement the full widget trait: [`Painter`] and
[`Controller`].

### Painter

The [`Painter`] widget lets you draw arbitrary custom content, but cannot
respond to events or otherwise contain update logic. Its general use is to
either provide a custom background to some other widget, or to implement
something like an icon or another graphical element that will be contained in
some other widget.

For instance, if we had some color data and we wanted to display it as a swatch
with rounded corners, we could use a `Painter`:

```rust,noplaypen
{{#include ../book_examples/src/custom_widgets_md.rs:color_swatch}}
```

`Painter` uses all the space that is available to it; if you want to give it a
set size, you must pass it explicit contraints, such as by wrapping it in a
[`SizedBox`]:

```rust,noplaypen
{{#include ../book_examples/src/custom_widgets_md.rs:sized_swatch}}
```

One other useful thing about `Painter` is that it can be used as the background
of a [`Container`] widget. If we wanted to have a label that used our swatch
as a background, we could do:

```rust,noplaypen
{{#include ../book_examples/src/custom_widgets_md.rs:background_label}}
```

(This uses the [`background`] method on [`WidgetExt`] to embed our label in a
container.)

### Controller

The [`Controller`] trait is sort of the inverse of `Painter`; it is a way to
make widgets that handle events, but don't do any layout or drawing. The idea
here is that you can use some `Controller` type to customize the behaviour of
some set of children.

The [`Controller`] trait has `event`, `update`, and `lifecycle` methods, just
like [`Widget`]; it does not have `paint` or `layout` methods. Also unlike
[`Widget`], all of its methods are optional; you can override only the method
that you need.

There's one other difference to the `Controller` methods; it is explicitly
passed a mutable reference to its child in each method, so that it can modify it
or forward events as needed.

As an arbitrary example, here is how you might use a `Controller` to make a
textbox fire some action (say doing a search) 300ms after the last keypress:

```rust,noplaypen
{{#include ../book_examples/src/custom_widgets_md.rs:annoying_textbox}}
```

## todo

v controller, painter
- how to do layout
    - how constraints work
    - child widget, set_layout_rect
    - paint bounds
- container widgets
- widgetpod & architecture
- commands and widgetid
- focus / active / hot
- request paint & request layout
- changing widgets at runtime

[`Controller`]: https://docs.rs/druid/0.6.0/druid/widget/trait.Controller.html
[`Widget`]: ./widget.md
[`Painter`]: https://docs.rs/druid/0.6.0/druid/widget/struct.Painter.html
[`SizedBox`]: https://docs.rs/druid/0.6.0/druid/widget/struct.SizedBox.html
[`Container`]: https://docs.rs/druid/0.6.0/druid/widget/struct.Container.html
[`WidgetExt`]: https://docs.rs/druid/0.6.0/druid/trait.WidgetExt.html
[`background`]: https://docs.rs/druid/0.6.0/druid/trait.WidgetExt.html#background
