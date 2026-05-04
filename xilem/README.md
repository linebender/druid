# An experimental Rust architecture for reactive UI

Note: this README is a bit out of date. To understand more of what's going on, please read the blog post, [Xilem: an architecture for UI in Rust].

This repo contains an experimental architecture, implemented with a toy UI. At a very high level, it combines ideas from Flutter, SwiftUI, and Elm. Like all of these, it uses lightweight view objects, diffing them to provide minimal updates to a retained UI. Like SwiftUI, it is strongly typed.

## Overall program flow

Like Elm, the app logic contains *centralized state.* On each cycle (meaning, roughly, on each high-level UI interaction such as a button click), the framework calls a closure, giving it mutable access to the app state, and the return value is a *view tree.* This view tree is fairly short-lived; it is used to render the UI, possibly dispatch some events, and be used as a reference for *diffing* by the next cycle, at which point it is dropped.

We'll use the standard counter example. Here the state is a single integer, and the view tree is a column containing two buttons.

```rust
fn app_logic(data: &mut u32) -> impl View<u32, (), Element = impl Widget> {
    Column::new((
        Button::new(format!("count: {}", data), |data| *data += 1),
        Button::new("reset", |data| *data = 0),
    ))
}
```

These are all just vanilla data structures. The next step is diffing or reconciling against a previous version, now a standard technique. The result is an *element tree.* Each node type in the view tree has a corresponding element as an associated type. The `build` method on a view node creates the element, and the `rebuild` method diffs against the previous version (for example, if the string changes) and updates the element. There's also an associated state tree, not actually needed in this simple example, but would be used for memoization.

The closures are the interesting part. When they're run, they take a mutable reference to the app data.

## Components

A major goal is to support React-like components, where modules that build UI for some fragment of the overall app state are composed together. 

```rust
struct AppData {
    count: u32,
}

fn count_button(count: u32) -> impl View<u32, (), Element = impl Widget> {
    Button::new(format!("count: {}", count), |data| *data += 1)
}

fn app_logic(data: &mut AppData) -> impl View<AppData, (), Element = impl Widget> {
    Adapt::new(|data: &mut AppData, thunk| thunk.call(&mut data.count),
        count_button(data.count))
}
```

This adapt node is very similar to a lens (quite familiar to existing Druid users), and is also very similar to the [Html.map] node in Elm. Note that in this case the data presented to the child component to render, and the mutable app state available in callbacks is the same, but that is not necessarily the case.

## Memoization

In the simplest case, the app builds the entire view tree, which is diffed against the previous tree, only to find that most of it hasn't changed.

When a subtree is a pure function of some data, as is the case for the button above, it makes sense to *memoize.* The data is compared to the previous version, and only when it's changed is the view tree build. The signature of the memoize node is nearly identical to [Html.lazy] in Elm:

```rust
fn app_logic(data: &mut AppData) -> impl View<AppData, (), Element = impl Widget> {
    Memoize::new(data.count, |count| {
        Button::new(format!("count: {}", count), |data: &mut AppData| {
            data.count += 1
        })
    }),
}
```

The current code uses a `PartialEq` bound, but in practice I think it might be much more useful to use pointer equality on `Rc` and `Arc`.

The combination of memoization with pointer equality and an adapt node that calls [Rc::make_mut] on the parent type is actually a powerful form of change tracking, similar in scope to Adapton, self-adjusting computation, or the types of binding objects used in SwiftUI. If a piece of data is rendered in two different places, it automatically propagates the change to both of those, without having to do any explicit management of the dependency graph.

I anticipate it will also be possible to do dirty tracking manually - the app logic can set a dirty flag when a subtree needs re-rendering.

## Optional type erasure

By default, view nodes are strongly typed. The type of a container includes the types of its children (through the `ViewTuple` trait), so for a large tree the type can become quite large. In addition, such types don't make for easy dynamic reconfiguration of the UI. SwiftUI has exactly this issue, and provides [AnyView] as the solution. Ours is more or less identical.

The type erasure of View nodes is not an easy trick, as the trait has two associated types and the `rebuild` method takes the previous view as a `&Self` typed parameter. Nonetheless, it is possible. (As far as I know, Olivier Faure was the first to demonstrate this technique, in [Panoramix], but I'm happy to be further enlightened)

[Html.lazy]: https://guide.elm-lang.org/optimization/lazy.html
[Html map]: https://package.elm-lang.org/packages/elm/html/latest/Html#map
[Rc::make_mut]: https://doc.rust-lang.org/std/rc/struct.Rc.html#method.make_mut
[AnyView]: https://developer.apple.com/documentation/swiftui/anyview
[Panoramix]: https://github.com/PoignardAzur/panoramix
[Xilem: an architecture for UI in Rust]: https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html