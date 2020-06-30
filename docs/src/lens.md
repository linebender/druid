# Lenses and the `Lens` trait

Let's say we're building a todo list application, and we are designing the widget
that will represent a single todo item. Our data model looks like this:

```rust,noplaypen
{{#include ../book_examples/src/lens_md.rs:todo_item}}
```

We would like our widget to display the title of the item, and then below
that to display two checkmarks that toggle the 'completed' and 'urgent' bools.
`Checkbox` (a widget included in Druid) implements `Widget<bool>`.
How do we use it with `TodoItem`? By using a `Lens`.

## Conceptual

You can think of a lens as a way of "focusing in" on one part of the data. You
have a `TodoItem`, but you *want* a `bool`.

`Lens` is a trait for types that perform this "focusing in" (aka *lensing*).
A simplified version of the `Lens` trait might look like this:

```rust,noplaypen
{{#include ../book_examples/src/lens_md.rs:simple_lens}}
```

That is, this type takes an instance of `In`, and returns an instance of `Out`.

For instance, imagine we wanted a lens to focus onto the `completed` state of
our `TodoItem`. With our simple trait, we might do:

```rust,noplaypen
{{#include ../book_examples/src/lens_md.rs:completed_lens}}
```

> **Note**: `Lens` isn't that helpful on its own; in Druid it is generally used alongside
`LensWrap`, which is a special widget that uses a `Lens` to change the `Data`
type of its child. Lets say we have a `Checkbox`, but our data is a `TodoItem`:
we can do, `LensWrap::new(my_checkbox, CompletedLens)` in order to bridge the
gap.

Our example is missing out on an important feature of lenses, though, which is that
they allow mutations that occur on the *lensed* data to propagate back to the
source. For this to work, lenses actually work with closures. The real signature
of `Lens` looks more like this (names changed for clarity):

```rust,noplaypen
{{#include ../book_examples/src/lens_md.rs:lens}}
```

Here `In` refers to the input to the `Lens` and `Out` is the output. `F` is a
closure that can return a result, `R`.

Now, instead of just being passed `Out` directly from the function, we pass the
function a closure that will *itself* be passed an `Out`; if our closure returns
a result, that will be given back to us.

This is unnecessary in the case of non-mutable access, but it is important for
mutable access, because in many circumstances (such as when using an `Rc` or
`Arc`) accessing a field mutably is expensive even if you don't do any mutation.

In any case, the real implementation of our lens would look like,

```rust,noplaypen
{{#include ../book_examples/src/lens_md.rs:completed_lens_real}}
```

That seems pretty simple and fairly annoying to write, which is why you
generally don't have to.

## Deriving lenses

For simple field access, you can `derive` the `Lens` trait.

```rust,noplaypen
{{#include ../book_examples/src/lens_md.rs:todo_item_lens}}
```

This handles the boilerplate of writing a lens for each field. It also does
something slightly sneaky: it exposes the generated lenses through the type
itself, as associated constants. What this means is that if you want to use the
lens that gives you the `completed` field, you can access it via
`TodoItem::completed`. The generated code basically looks something like:

```rust, noplaypen
struct GeneratedLens_AppData_title;
struct GeneratedLens_AppData_completed;
struct GeneratedLens_AppData_urgent;

impl TodoItem {
    const title = GeneratedLens_AppData_title;
    const completed = GeneratedLens_AppData_completed;
    const urgent = GeneratedLens_AppData_urgent;
}
```

One consequence of this is that if your type has a method with the same name as
one of its fields, `derive` will fail. To get around this, you can specify a
custom name for a field's lens:

```rust,noplaypen
{{#include ../book_examples/src/lens_md.rs:lens_name}}
```

## Using lenses

The easiest way to use a lens is with the `lens` method that is provided through
the `WigetExt` trait; this is a convenient way to wrap a widget in a `LensWrap`
with a given lens.

Let's build the UI for our todo list item:

```rust,noplaypen
{{#include ../book_examples/src/lens_md.rs:build_ui}}
```

## Advanced lenses

Field access is a very simple (and common, and *useful*) case, but lenses can do much more than that.

### `LensExt` and combinators

Similar to the `WidgetExt` trait, we offer a `LensExt` trait that provides
various functions for composing lenses. These are similar to the various methods
on iterator; you can `map` from one lens to another, you can index into a
collection, or you can efficiently access data in an `Arc` without unnecessary
mutation; see the main crate documentation for more.

As your application gets more complicated, it will become likely that you want
to use fancier sorts of lensing, and `map` and company can start to get out
of hand; when that happens, you can always implement a lens by hand.

### Getting something from a collection

Your application is a contact book, and you would like a lens that
focuses on a specific contact. You might write something like this:

```rust,noplaypen
{{#include ../book_examples/src/lens_md.rs:contact}}
```

### Doing a conversion

What if you have a distance in miles that you would like to display in
kilometres?

```rust,noplaypen
{{#include ../book_examples/src/lens_md.rs:conversion}}
```
