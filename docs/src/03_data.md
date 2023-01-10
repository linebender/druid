# Dataflow and the `Data` trait

The Druid architecture is based on a two-way dataflow.

At the root level, you define the application state, which is passed to each child widget as associated data. Some Widgets (eg LensWrap) will only pass a subset of that data to their children.

Some widgets (eg Button, TextBox, Checkbox) can mutate the data passed to them by their parents in reaction to user events. The data mutated in a child widget is also changed in the parent widgets, all the way to the root.

When you mutate a widget's associated data, Druid compares the old and new version, and propagates the change to the widgets that are affected by the change.

Note that, in all that workflow, Widgets don't actually store their associated data. A `Button<Vector<String>>` doesn't actually store a `Vector<String>`, instead the framework stores one per button, which is provided to widget methods.

For this to work, your model must implement the `Clone` and `Data` traits. The `Data` trait has a single method:

```rust,no_run,noplaypen
{{#include ../../druid/src/data.rs:same_fn}}
```

This method checks for equality, but allows for false negatives.


## Performance

It is important that your data is cheap to clone and cheap to compare; we encourage the use of reference counted pointers to allow cheap cloning of more expensive types. `Arc` and `Rc` have blanket `Data` impls that do pointer comparison, so if you have a type that does not implement `Data`, you can always just wrap it in one of those smart pointers.

### Collections

`Data` is expected to be cheap to clone and cheap to compare, which can cause
issues with collection types. For this reason, `Data` is not implemented for
`std` types like `Vec` or `HashMap`.

You can always put these types inside an `Rc` or an `Arc`, or if you're dealing with
larger collections you can build Druid with the `im` feature, which brings in
the [`im` crate], and adds a `Data` impl for the collections there. The [`im`
crate] is a collection of immutable data structures that act a lot like the `std`
collections, but can be cloned efficiently.


## Derive

`Data` can be derived. This is recursive; it requires `Data` to be implemented
for all members. For 'C style' enums (enums where no variant has any fields)
this also requires an implementation of `PartialEq`. `Data` is implemented for
a number of `std` types, including all primitive types, `String`, `Arc`, `Rc`,
as well as `Option`, `Result`, and various tuples whose members implement
`Data`.

Here is an example of using `Data` to implement a simple data model:

```rust
{{#include ../book_examples/src/data_md.rs:derive}}
```

[`im` crate]: https://docs.rs/im


## Mapping `Data` with lenses

In Druid, most container widgets expect their children to have the same associated data. If you have a `Flex<Foobar>`, you can only append widgets that implement `Widget<Foobar>` to it.

In some cases, however, you want to compose widgets that operate on different subsets of the data. Maybe you want to add two widgets to the above Flex, one that uses the field `foo` and another that uses the field `bar`, and they might respectively implement `Widget<Foo>` and `Widget<Bar>`.

Lenses allow you to bridge that type difference. A lens is a type that represents a *two-way* mapping between two data types. That is, a lens from X to Y can take an instance of X and give you an instance of Y, and can take a modified Y and apply the modification to X.

To expand on our Foobar example:

```rust
#[derive(Lens)]
struct Foobar {
    foo: Foo,
    bar: Bar,
}
```

The derive macro above generates two lenses: `Foobar::foo` and `Foobar::bar`. `Foobar::foo` can take an instance of `Foobar` and give you a shared or mutable reference to its `foo` field. Finally, the type `LensWrap` can take that lens and use it to map between different widget types:

```rust
fn build_foo() -> impl Widget<Foo> {
    // ...
}

fn build_bar() -> impl Widget<Bar> {

}

fn build_foobar() -> impl Widget<Foobar> {
    Flex::column()
        .with_child(
            LensWrap::new(build_foo(), Foobar::foo),
        )
        .with_child(
            LensWrap::new(build_bar(), Foobar::bar),
        )
}
```

See the Lens chapter for a more in-depth explanation of what lenses are and how they're implemented.
