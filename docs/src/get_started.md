# Get started with Druid
This chapter will walk you through setting up a simple druid application from start to finish.

## Set up a Druid project
Setting up a project is a simple as creating a new Rust project;
```bash
> cargo new druid-example
```

And then adding druid as a dependency to Cargo.toml
```toml
[dependencies]
druid = "0.4.0"
```

To show a minimal window with a label replace `main.rs` with this;
```rust
use druid::{AppLauncher, WindowDesc, Widget};
use druid::widget::Label;
use druid::shell::Error;

fn build_ui() -> impl Widget<()> {
    Label::new("Hello world")
}

fn main() -> Result<(), Error> {
    AppLauncher::with_window(WindowDesc::new(build_ui)).launch(())?;
    Ok(())
}
```
In our main function we create an `AppLauncher`, pass it a `WindowDesc` that wraps build_ui function and launch it. Druid will use `build_ui` to build and rebuild our main window every time it needs to refresh. `build_ui` returns a tree of widgets. For now this tree consists of one simple label widget.

This is a very simple example application and it's missing some important pieces. We will add these in the coming few paragraphs.

## Draw more widgets
The first thing we could do to make our example application more interesting is to draw more than one widget. Unfortunately `WindowDesc::new` expects a function that returns only one Widget. We also need a way to tell Druid how to lay-out our widgets.
Luckilly we can solve both these problems at the same time by. Many widgets can take child widgets. This way they form a widget-tree with one widget at the base. The widgets higher up in the tree know how to lay-out their children. That way we describe a window as a widget-tree with layout containers as the branches and widgets as the leaves. The top container widget can then be returned as a single `impl Widget` from `build_ui`

To see how this works we will divide our window in four, two rows and two columns with a single label in each of the quadrants. We can lay-out our labels using the `Flex` widget.

```rust
fn build_ui() -> impl Widget<()> {
    Flex::row()
        .with_child(
            Flex::column()
                .with_child(Label::new("top left"), 1.0)
                .with_child(Label::new("bottom left"), 1.0), 
            1.0)
        .with_child(
            Flex::column()
                .with_child(Label::new("top right"), 1.0)
                .with_child(Label::new("bottom right"), 1.0),
            1.0)
}
```

This looks nice but the labels on the left are drawn right against the window edge, so we needs some padding. We also want to center the two bottom labels.
Unlike many other UI frameworks, widgets in Druid do have padding or alignment properties. Widgets are kept as simple as possible. To add padding you simply wrap the labels in a `Padding` widget. Centering widgets is done using the `Align` widget.

```rust
fn build_ui() -> impl Widget<()> {
    Padding::new(
        10.0,
        Flex::row()
            .with_child(
                Flex::column()
                    .with_child(Label::new("top left"), 1.0)
                    .with_child(centered_label("bottom left"), 1.0), 
                1.0)
            .with_child(
                Flex::column()
                    .with_child(Label::new("top right"), 1.0)
                    .with_child(centered_label("bottom right"), 1.0),
                1.0))
}

fn centered_label(text: &str) -> impl Widget<()> {
    Align::centered(Label::new(text))
}
```

Do not forget to import the new widgets;
```rust
use druid::widget::{Label, Flex, Padding, Align};
```

## Handle user input


## Maintaining state


## Putting it all together
