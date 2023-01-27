# Get started with Druid

This chapter will walk you through setting up a simple Druid application from start to finish.


## Setting up Druid dependencies

If you're on Linux or OpenBSD, you'll need to install GTK-3's development kit first.

### Linux
On Linux, Druid requires gtk+3.

On Ubuntu this can be installed with
```sh
> sudo apt-get install libgtk-3-dev
```

On Fedora
```sh
> sudo dnf install gtk3-devel glib2-devel
```

See [GTK installation page] for more installation instructions.

### OpenBSD
On OpenBSD, Druid requires gtk+3;  install from packages:
```sh
> pkg_add gtk+3
```


## Starting a project

Create a new cargo binary crate, and add `druid` as a dependency:

```sh
> cargo new my-druid-app
      Created binary (application) `my-druid-app` package
> cd my-druid-app
> cargo add druid
```

You should now have a stub of a project:

```sh
> tree
.
├── Cargo.lock
├── Cargo.toml
└── src
    └── main.rs
```

## Hello world

To show a minimal window with a label, write the following code in your `main.rs`:

```rust, noplaypen
{{#include ../book_examples/src/getting_started_md.rs:example_1_imports}}

{{#include ../book_examples/src/getting_started_md.rs:example_1}}
```

In our main function we create an `AppLauncher`, pass it a `WindowDesc`, and launch it. We use `build_ui` to create a tree of widgets to pass to our `WindowDesc`. For now this tree consists of one simple label widget.

This is a very simple example application, using only the bare minimum of features. We can do something more complex.


## Add more widgets

The first thing we could do to make our example application more interesting is to display more than one widget. However, `WindowDesc::new` expects a function that returns only one Widget. We also need a way to tell Druid how to lay out our widgets.

What we need to do is initialize our `WindowDesc` with a widget tree, with a single widget at the root. Some widgets can have children, and know how to lay them out; these are called container widgets.

We describe our window as a widget tree with container widgets as nodes, and label widgets as the leaves. Our `build_ui` function is then responsible for building this widget tree.

As an example, we'll build a todo-list app. At first, this app will have two columns, one with the list, and one with a placeholder for a button, each in a box with visible borders. We'll need to use the `Split`, `Flex` and `Container` widgets:

```rust, noplaypen
{{#include ../book_examples/src/getting_started_md.rs:example_2_imports}}

// ...

fn build_ui() -> impl Widget<()> {
{{#include ../book_examples/src/getting_started_md.rs:example_2_builder}}
}
```

We get a UI which is starting to look like a real application. Still, it's inherently static. We would like to add some interactivity, but before we can do that, our next step will be to make the UI data-driven.


## Widget data

You may have noticed that our `build_ui()` function returns `impl Widget<()>`. This syntax describes an existential type which implements the `Widget` trait, with a generic parameter.

This generic parameter is the Widget's data. Since our UI so far has been stateless, the data is the unit type. But since we're writing a todo-list, we'll want our widget to depend on the list data:

```rust, noplaypen
{{#include ../book_examples/src/getting_started_md.rs:example_3_imports}}
type TodoList = Vector<String>;

// ...

fn build_ui() -> impl Widget<TodoList> {
    // ...
}
```

Here we're using a Vector from the `im` crate; for reasons we'll get into later, we can't use the standard library's Vec as our data. But `im::Vector` is functionally equivalent to `std::vec::Vec`.

To build a UI that changes depending on our widget data, we use the `List` widget, and `Label::dynamic`:

```rust, noplaypen
{{#include ../book_examples/src/getting_started_md.rs:example_3b_imports}}

// ...

fn build_ui() -> impl Widget<TodoList> {
{{#include ../book_examples/src/getting_started_md.rs:example_3_builder}}
}
```

List is a special widget that takes a collection as data, and creates one widget with per collection item, with the item as data. In other words, our `List` implements `Widget<Vector<String>>` while the label returned by `Label::dynamic` implements `Widget<String>`. This is all resolved automatically by type inference.

`Label::dynamic` creates a label whose content depends on the data parameter.

Now, to test our UI, we can launch it with a hardcoded list:

```rust, noplaypen
{{#include ../book_examples/src/getting_started_md.rs:example_4_imports}}

// ...

fn main() {
{{#include ../book_examples/src/getting_started_md.rs:example_4_main}}
}
```

We can now change the contents of the UI depending on the data we want to display; but our UI is still static. To add user interaction, we need a way to modify our data.


## Interaction widgets

First, to interact with our UI, we add a button:


```rust, noplaypen
{{#include ../book_examples/src/getting_started_md.rs:example_5_imports}}

// ...

fn build_ui() -> impl Widget<TodoList> {
    // ...

{{#include ../book_examples/src/getting_started_md.rs:example_5a_button}}

    // ...
}
```

If you build this, you'll notice clicking the button doesn't do anything. We need to give it a callback, that will take the data as parameter and mutate it:

```rust, noplaypen
fn build_ui() -> impl Widget<TodoList> {
    // ...

{{#include ../book_examples/src/getting_started_md.rs:example_5b_button}}

    // ...
}
```

Now, clicking on the button adds an item to our list, but it always adds the same item. To change this, we need to add a textbox to our app, which will require that we make our data type a bit more complex.


### Selecting a structure's field with lenses

To complete our todo-list, we need to change our app data type. Instead of just having a list of strings, we need to have a list *and* a string representing the next item to be added:

```rust, noplaypen
{{#include ../book_examples/src/getting_started_2_md.rs:example_6_struct}}
```

However, now we have a problem: our List widget which expected a `Vector<...>` won't know how to handle a struct. So, we need to modify Druid's dataflow so that, given the TodoList above, the List widget will have access to the `items` field. This is done with a `Lens`, which we'll explain next chapter.

Furthermore, to pass our type as the a generic parameter to `Widget`, we need it to implement the `Data` trait (and `Clone`), more on that next chapter.

So, given the two requirements above, our declaration will actually look like:

```rust, noplaypen
{{#include ../book_examples/src/getting_started_2_md.rs:example_6_imports}}

{{#include ../book_examples/src/getting_started_2_md.rs:example_6_derive}}
{{#include ../book_examples/src/getting_started_2_md.rs:example_6_struct}}
```

Among other things, the above declaration defines two lenses, `TodoList::items` and `TodoList::next_item`, which take a TodoList as input and give a mutable reference to its `items` and `next_item` fields, respectively.

Next, we'll use the `LensWrap` widget wrapper to pass `items` to our `List` widget:

```rust, noplaypen
{{#include ../book_examples/src/getting_started_2_md.rs:example_7_imports}}

// ...

fn build_ui() -> impl Widget<TodoList> {
    // ...

{{#include ../book_examples/src/getting_started_2_md.rs:example_7}}

    // ...
}
```

We also need to modify the callback of our button:

```rust, noplaypen
fn build_ui() -> impl Widget<TodoList> {
    // ...

{{#include ../book_examples/src/getting_started_2_md.rs:example_7b}}

    // ...
}
```

Finally, we add a textbox to our widget with `TodoList::next_item` as its data:

```rust, noplaypen
{{#include ../book_examples/src/getting_started_2_md.rs:example_8_imports}}

// ...

fn build_ui() -> impl Widget<TodoList> {
    // ...

{{#include ../book_examples/src/getting_started_2_md.rs:example_8}}

    // ...
}
```

Now, when we push the button, whatever was in the textbox is added to the list.


## Putting it all together

If we pull all the code we have written so far, our `main.rs` now looks like this:

```rust, noplaypen
{{#include ../book_examples/src/getting_started_md.rs:example_1_imports}}
{{#include ../book_examples/src/getting_started_md.rs:example_2_imports}}
{{#include ../book_examples/src/getting_started_md.rs:example_3b_imports}}
{{#include ../book_examples/src/getting_started_md.rs:example_3_imports}}
{{#include ../book_examples/src/getting_started_md.rs:example_4_imports}}
{{#include ../book_examples/src/getting_started_md.rs:example_5_imports}}
{{#include ../book_examples/src/getting_started_2_md.rs:example_6_imports}}
{{#include ../book_examples/src/getting_started_2_md.rs:example_7_imports}}
{{#include ../book_examples/src/getting_started_2_md.rs:example_8_imports}}

{{#include ../book_examples/src/getting_started_2_md.rs:complete_code}}
```

We now have a list of items, which we can add to by filling a textbox and clicking a button.

[GTK installation page]: https://www.gtk.org/docs/installations/linux/
