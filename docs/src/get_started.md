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
druid = "0.3.2"
```

To show a minimal window we put the following in main.rs;
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

This application uses the AppLauncher to show a window. We have built a build_ui function that returns our 

 For now this function is a simple lambda that returns a Label.

This is probably the simpelest Druid program that will actually work, and it's missing some important pieces. We will add these in the coming few paragraphs.

## Draw more widgets


## Maintaining state


## Handling user input
