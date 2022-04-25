# Get started with Druid
This chapter will walk you through setting up a simple Druid application from start to finish.

## Set up a Druid project
Setting up a project is a simple as creating a new Rust project;
```bash
> cargo new druid-example
```

And then adding Druid as a dependency to Cargo.toml
```toml
[dependencies]
druid = { git = "https://github.com/linebender/druid.git" }
```

To show a minimal window with a label replace `main.rs` with this;
```rust, noplaypen
use druid::{AppLauncher, WindowDesc, Widget, PlatformError};
use druid::widget::Label;

fn build_ui() -> impl Widget<()> {
    Label::new("Hello world")
}

fn main() -> Result<(), PlatformError> {
    AppLauncher::with_window(WindowDesc::new(build_ui())).launch(())?;
    Ok(())
}
```
In our main function we create an `AppLauncher`, pass it a `WindowDesc`, and launch it. We use `build_ui` to create a tree of widgets to pass to our `WindowDesc`. For now this tree consists of one simple label widget.

This is a very simple example application and it's missing some important pieces. We will add these in the coming few paragraphs.

## Draw more widgets
The first thing we could do to make our example application more interesting is to draw more than one widget. Unfortunately `WindowDesc::new` expects a function that returns only one Widget. We also need a way to tell Druid how to lay-out our widgets.
We solve both these problems by passing in a widget-tree with one single widget at the top. Widgets can have children and widgets higher up in the tree know how to lay-out their children. That way we describe a window as a widget-tree with layout containers as the branches and widgets as the leaves. Our `build_ui` function is then responsible for building this widget tree.

To see how this works we will divide our window in four. We'll have two rows and two columns with a single label in each of the quadrants. We can lay-out our labels using the `Flex` widget.

```rust, noplaypen
fn build_ui() -> impl Widget<()> {
    Flex::row()
        .with_flex_child(
            Flex::column()
                .with_flex_child(Label::new("top left"), 1.0)
                .with_flex_child(Label::new("bottom left"), 1.0),
            1.0)
        .with_flex_child(
            Flex::column()
                .with_flex_child(Label::new("top right"), 1.0)
                .with_flex_child(Label::new("bottom right"), 1.0),
            1.0)
}
```

This looks nice but the labels on the left are drawn right against the window edge, so we needs some padding. Lets say we also want to center the two bottom labels. Unlike many other UI frameworks, widgets in Druid don't have padding or alignment properties themselves. Widgets are kept as simple as possible.

Features like padding or alignment are implemented in separate widgets. To add padding you simply wrap the labels in a `Padding` widget. Centering widgets is done using the `Align` widget set to `centered`.

```rust, noplaypen
fn build_ui() -> impl Widget<()> {
    Padding::new(
        10.0,
        Flex::row()
            .with_flex_child(
                Flex::column()
                    .with_flex_child(Label::new("top left"), 1.0)
                    .with_flex_child(Align::centered(Label::new("bottom left")), 1.0),
                1.0)
            .with_flex_child(
                Flex::column()
                    .with_flex_child(Label::new("top right"), 1.0)
                    .with_flex_child(Align::centered(Label::new("bottom right")), 1.0),
                1.0))
}
```

Do not forget to import the new widgets;
```rust, noplaypen
use druid::widget::{Label, Flex, Padding, Align};
```

But this is can get too verbose, so there are helper functions, such as `center()`, `padding(x)`, etc.

They take the widget and wrap it into another widget for you. This makes it easy to chain different features, like this: 
```rust, noplaypen
Label::new("foo").center().padding(5.0).fix_height(42.0).border(Color::RED, 5.0)
```

Here's how it would look like in our example:
```rust, noplaypen
fn build_ui() -> impl Widget<()> {
    Flex::row()
        .with_flex_child(
            Flex::column()
                .with_flex_child(Label::new("top left"), 1.0)
                .with_flex_child(Label::new("bottom left").center(), 1.0),
            1.0)
        .with_flex_child(
            Flex::column()
                .with_flex_child(Label::new("top right"), 1.0)
                .with_flex_child(Label::new("bottom right").center(), 1.0),
            1.0)
        .padding(10.0)
}
```
This does not require importing `Padding` or `Align`, but requires importing `druid::WidgetExt`.



## Application state
We can display a window and draw and position widgets in it. Now it's time to find out how we can tie these widgets to
the rest of our application. First lets see how we can display information from our application in the user interface.
For this we need to define what our application's state looks like.

```rust, noplaypen
use druid::Data;

#[derive(Data, Clone)]
struct AppState {
    some_text: String,
}
```

Your application state struct needs to implement `Data` (which can be derived and requires `Clone`).
Members can be anything, but it's recommended to use types that already implement `Data`. Check [`Data` trait] section for more info.

Now we want to use our state to drive what gets displayed.
```rust, noplaypen
fn main() -> Result<(), PlatformError> {
    let state = AppState {
        label_text: String::from("String stored in AppState"),
    };
    AppLauncher::with_window(WindowDesc::new(build_ui())).launch(state)?;
    Ok(())
}

fn build_ui() -> impl Widget<AppState> {
    Flex::column()
        .with_child(Label::dynamic(|data: &AppState, _env| data.label_text.clone()))
        .padding(10.0)
}
```
In `main()` we initialize the state and pass it to the `AppLauncher`.

We changed the return type of the `build_ui` function. Now we say that our widget operates on data that is `AppState`. This struct is (usually) passed down to children of widgets in various functions.

Then, we create a dynamic `Label`, which takes a closure that gets a reference to the `AppState`.
In this closure we can do whatever we need and return a `String` that will be displayed in the `Label`.
Right now we just take the `label_text` member of `AppState` and clone it.

## Handle user input
We can't achieve much without modifying the app state.

Here we will use `TextBox` widget to get input from the user.

```rust, noplaypen
fn build_ui() -> impl Widget<AppState> {
    Flex::column()
        .with_child(Label::dynamic(|data: &AppState, _env| data.label_text.clone()))
        .with_child(TextBox::new().lens(AppState::label_text))
        .padding(10.0)
}
```

You might notice that we used something new here: `.lens()` function.

You can learn more about lenses in [`Lens` trait] section, but the general gist is that lenses allow you to easily select a subset of your app state to pass down to the widget. Because `TextBox` usually does not require to know everything about the state, just the `String` that needs to be displayed and updated.

But to use this functionality like that, we need to derive `Lens` on our `AppState`, so now it looks like this:
```rust, noplaypen
#[derive(Data, Clone, Lens)]
struct AppState {
    label_text: String,
}
```
Remember to import `druid::Lens`.

Now user can edit the `label_text`!

## Putting it all together
Here's an example app that uses what we learned so far and greets the user.


```rust, noplaypen
#use druid::widget::prelude::*;
#use druid::widget::{Flex, Label, TextBox};
#use druid::{AppLauncher, Data, Lens, UnitPoint, WidgetExt, WindowDesc};
#
const VERTICAL_WIDGET_SPACING: f64 = 20.0;
const TEXT_BOX_WIDTH: f64 = 200.0;

#[derive(Clone, Data, Lens)]
struct HelloState {
    name: String,
}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_ui())
        .title("Hello World!")
        .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state: HelloState = HelloState {
        name: "World".into(),
    };

    // start the application. Here we pass in the application state.
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_ui() -> impl Widget<HelloState> {
    // a label that will determine its text based on the current app data.
    let label = Label::new(|data: &HelloState, _env: &Env| {
        if data.name.is_empty() {
            "Hello anybody!?".to_string()
        } else {
            format!("Hello {}!", data.name)
        }
    })
    .with_text_size(32.0);

    // a textbox that modifies `name`.
    let textbox = TextBox::new()
        .with_placeholder("Who are we greeting?")
        .with_text_size(18.0)
        .fix_width(TEXT_BOX_WIDTH)
        .lens(HelloState::name);

    // arrange the two widgets vertically, with some padding
    Flex::column()
        .with_child(label)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(textbox)
        .align_vertical(UnitPoint::CENTER)
}
```


[`Data` trait]: data.md
[`Lens` trait]: lens.md
