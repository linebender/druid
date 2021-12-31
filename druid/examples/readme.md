# Examples

There are several different kind of examples, some demonstrate one particular
Druid concept, some are tools used for testing and debugging, and
others are more complete examples of how to tie everything together.
The latter are listed separately under [showcases](#Showcases).

## Anim
```
cargo run --example anim
```
This example shows how to make a simple animation using `Event::AnimFrame`.

## Async Event
```
cargo run --example async_event
```
Demonstrates receiving data from some outside source, and updating the UI in response. This is similar to [blocking function](#Blocking Function) but here the data source is fully independent, and runs for the lifetime of the program.

## Blocking Function
```
cargo run --example blocking_function
```
Sometimes you need to fetch some data from disk or from the internet,
but you should never block the UI thread with long running operations!
Instead you should run this task in a separate thread, and have it send
you the data as it arrives. This is very similar to [async event](#Async Event)
 except the event is initiated by the main thread.

## Cursor
```
cargo run --example cursor --features="image png"
```
This example demonstrates how to set the cursor icon, and how to use a custom cursor.

## Custom Widget
```
cargo run --example custom_widget
```
This shows how to use all of the methods on `PaintCtx` used for drawing on a canvas.
You can use this to draw everything from text to images to curves.

## Either
```
cargo run --example either
```
This example shows how to use the `Either` widget, which shows one of two children based on some predicate.
This can be useful for loading screens, error messages, and other situations where you want to show one of
two possible widgets.

## Hello
```
cargo run --example hello
```
This shows some of the basics of druid. If you need a start of how to build an application with a text-box and some labels this is where to start.

## Hello_web
For more info and prerequistes see [druid/examples/hello_web/README.md](druid/examples/hello_web/README.md).
```
cd druid/examples/hello_web
wasm-pack build --out-dir pkg --out-name hello_web
```
[View at http://localhost:8000](http://localhost:8000].

This is an example of how to get almost any druid application can be used on the web. This is just the hello_world example but should work for all of them.

## Web
For more info and prerequistes see [druid/examples/web/README.md](druid/examples/web/README.md).
```
cd druid/examples/web
wasm-pack build --out-dir pkg --out-name web
```
[View at http://localhost:8000](http://localhost:8000].

Simple web app.

## Identity
```
cargo run --example identity
```
In druid identity is used to send specific widgets commands. Instead of a command going to all the widgets, you can send them to just the one you need. This example has some colorwells and some buttons that interact with them. All of them are identical, except the identity, which makes it possible for the buttons to only affect a single colorwell.

## Invalidation
```
cargo run --example invalidation --features="im"
```
A demonstration how to use debug invalidation regions in your own widgets, including some examples of builtin widgets.

## Layout
```
cargo run --example layout
```
An example of how basic widget composition works in druid. There are no custom widgets just compositions of builtin ones.

## Lens
```
cargo run --example lens
```
Lenses are a core part of druid, they allow you to zoom into a part of the app state.

## List
```
cargo run --example list --features="im"
```
This shows you how you could, for example, add items to lists and delete them. 

## Markdown Preview
```
cargo run --example markdown_preview
```
An example of markdown preview on the left side and editable text on the right side.

## Multiple Windows
```
cargo run --example multiwin
```
Having multiple windows is a super nice tool to have when developing applications. This shows you the basic setup you need for a second window.

## Open Save
```
cargo run --example open_save
```
Opening and saving files is crucial for a lot of applications. This shows you how to get opening and saving files working cross platform.

## Panels
```
cargo run --example panels
```
Very similar to [layout](#Layout) but it splits the screen into 2 segments

## Value Formatting

To run this example, make sure you are in `druid/examples/value_formatting`
And then run `cargo run`

Druid doesnt have numeric specific texboxes, instead you have to parse the input as if it were a numeric value.
This example shows you how to parse, and validate text input. 

## Split
```
cargo run --example split_demo
```

The split widget allows you to put multiple widgets next, or on top of each other.
This also allows the user to resize them.

## Scroll
```
cargo run --example scroll
```
Scrolling is a great way to show more content then can be displayed on the screen at a time. This is an example showing you how to use them.

## Split
```
cargo run --example split_demo
```
An example of how to split a widget in 2 in various ways. This also includes having the user drag the border!! 

## Sub Window
Not working, no sub-window seen?
```
cargo run --example sub_window
```

This shows you how to make a completely new window with shared state.

## Svg
```
cargo run --example svg --features="svg"
```
This shows you how to display an SVG as a widget.

## Switches
```
cargo run --example switches
```
Switches are useful in many ways, this example shows how to use the druid built-in ones. This includes on/off and up/down for incrementing numeric values.

## Tabs
```
cargo run --example tabs --features="im"
```
Tabs allow you to seperate different portions of the UI. This example shows you how to use them in druid. similar to [view switcher](#View Switcher) but with with a different purpose.

## Text
```
cargo run --example text
```
Text shows the effects of TextAlignment and LineBreaker types.

## TextBox
```
cargo run --example textbox
```
Textbox demostrates some of the possible configuraitons of the TextBox widget.

## Timer
```
cargo run --example timer
```
Timers allow you to send events to your widgets at a certain points inthe future. This example shows how to use them.

## Transparency
```
cargo run --example transparency
```
This shows you how to make the window transparent, so the rest of the desktop shows behind it.

## View Switcher
```
cargo run --example view_switcher
```
Very similar to [tabs](#Tabs) but this allows you to have more control over it. This allows you to switch out widgets on the fly.

# Showcases

## Calc
```
cargo run --example calc
```

## Disabled
```
cargo run --example disabled
```

This showcases all the widgets that can have disabled input. Disabling a widget is usefull for preventing the user from entering input.

## Event Viewer
```
cargo run --example event_viewer
```

Used as a debugging tool, this prints out mouse and keyboard events as they are received by Druid.

## Flex
```
cargo run --example flex
```

Flex shows off all the things you can do with flex elements. You can play with all the setings and it will change in real-time.

## Game Of Life
```
cargo run --example game_of_life
```

A simple implementation of Conway's game of life. You can change the evolution speed, and pauze so you can take your time making your own creations!

## Image
```
cargo run --example image --features "image png"
```

Image shows off all the knobs you can turn on images. You can play with them with real time results, which you to figure out what settings are best for you.

Please note that the image is exported with some kind of interpolation. So even when you turn interpolation off/NearestNeighbor in druid, you will still see this because that's how the image actually looks.

## Scroll Colors
```
cargo run --example scroll_colors
```

This is a showcase is scrolling through an image gradient square. The square is divided into smaller squares each with a unique color. There are other ways to to this like one big widget with an image for example.

## Styled Text
```
cargo run --example styled_text
```

In druid you can change all kinds of styling aspects of text as not all text should look
the same. This showcases some of those things such as, color, size, and monospace.

## Widget Gallery
```
cargo run --example widget_gallery --features="svg im image png"
```

This is a showcase of some simple widgets with their default styling. These are interactive, but you cannot change any of their styling.
