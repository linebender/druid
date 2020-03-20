# Lenses

Lets say we're building a todo list application, and we are designing the widget
that will represent a single todo item. Or data model looks like this:

```rust
/// A single todo item.
#[derive(Clone, Data)]
struct TodoItem {
    title: String,
    completed: bool,
    urgent: bool,
}
```

And we would like our widget to display the title of the item, and then below
that to display two checkmarks that toggle the 'completed' and 'urgent' bools.
`Checkbox` is a struct that implements `Widget<bool>`. How do we use it with
`TodoItem`? By using a `Lens`.

## Conceptual

You can think of a lens as a way of "focusing in" on one part of the data. You
have a `TodoItem`, but you *want* a `bool`.

`Lens` is a trait for types that perform this "focusing in" (aka *lensing*).
A simplified version of the `Lens` trait might look like this:

```rust
trait SimpleLens<T, U> {
    fn focus(&self, data: &T) -> U;
}
```

That is, this type takes an instance of `T`, and returns an instance of `U`.

For instance, imagine we wanted a lens to focus onto the `completed` state of
our `TodoItem`. With our simple trait, we might do:

```rust
/// This is the type of the lens itself; in this case it has no state.
struct CompletedLens;

impl SimpleLens<TodoItem, bool> for CompletedLens {
    fn focus(&self, data: &TodoItem) -> bool {
        data.completed
    }
}
```

> **Note**: `Lens` isn't that helpful on its own; in druid it is generally used alongside
`LensWrap`, which is a special widget that uses a `Lens` to change the `Data`
type of its child. Lets say we have a `Checkbox`, but our data is a `TodoItem`:
we can do, `LensWrap::new(my_checkbox, CompletedLens)` in order to bridge the
gap.

Our example is missing out on an important feature of lenses, though, which is that
they allow mutations that occur on the *lensed* data to propagate back to the
source. For this to work, lenses actually work with closures. The real signature
of `Lens` looks more like this (names changed for clarity):

```rust
pub trait Lens<In, Out> {
    /// Get non-mut access to the field.
    fn with<R, F: FnOnce(&Out) -> R>(&self, data: &In, f: F) -> R;
    /// Get mut access to the field.
    fn with_mut<R, F: FnOnce(&mut Out) -> R>(&self, data: &mut In, f: F) -> R;
}
```

Here `In` refers to the input to the `Lens` and `Out` is the output. `F` is a
closure that can return a result, `R`.

Now, instead of just being passed `Out` directly from the function, we pass the
function a closure that will *itself* be passed an `Out`; if our closure returns
a result, that will be given back to us.

This is unnecessary in the case of non-mutable access, but it is important for
mutable access, because in many circumstances (such as when using an `Rc` or
`Arc`) accessing a field mutably is expensive even if you don't do any mutation.

In any case, the real implemntation of our lens would look like,

```rust
struct CompletedLens;

impl Lens<TodoItem, bool> for CompletedLens {
    fn with<R, F: FnOnce(&bool) -> R>(&self, data: &TodoItem, f: F) -> R {
        f(&data.completed)
    }

    fn with_mut<R, F: FnOnce(&mut bool) -> R>(&self, data: &mut TodoItem, f: F) -> R {
        f(&mut data.completed)
    }
}
```

That seems pretty simple and fairly annoying to write, which is why you
generally don't have to.

## Deriving lenses

For simple field access, you can `derive` the `Lens` trait.

```rust
/// A single todo item.
#[derive(Clone, Data, Lens)]
struct TodoItem {
    title: String,
    completed: bool,
    urgent: bool,
}
```

This handles the boilerplate of writing a lens for each field. It also does
something slightly sneaky: it exposes the generated lenses through the type
itself, as associated constants. What this means is that if you want to use the
lens that gives you the `completed` field, you can access it via
`TodoItem::completed`. The generated code basically looks something like:

```rust
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

```rust
#[derive(Lens)]
struct Item {
    #[druid(lens_name = "count_lens")]
    count: usize,
}

// this now works
impl Item {
    fn count(&self) -> usize { self.count }
}
```

## Using lenses

The easiest way to use a lens is with the `lens` method that is provided through
the `WigetExt` trait; this is a convenient way to wrap a widget in a `LensWrap`
with a given lens.

Let's build the UI for our todo list item:

```rust
use druid::widget::{Checkbox, Flex, Label, WidgetExt};
use druid::{Data, Lens, Widget};

#[derive(Clone, Data, Lens)]
struct TodoItem {
    title: String,
    completed: bool,
    urgent: bool,
}

fn make_todo_item() -> impl Widget<TodoItem> {
    // A label that generates its text based on the data
    let title = Label::dynamic(|text, _| text.to_string()).lens(TodoItem::title);
    let completed = Checkbox::new().lens(TodoItem::completed);
    let urgent = Checkbox::new().lens(TodoItem::urgent);

    Flex::column()
        // label on top
        .with_child(title)
        // two checkboxes below
        .with_child(Flex::row().with_child(completed).with_child(urgent))
}
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

### getting something from a collection

Let say your application is a contact book, and you would like a lens that
focuses on a specific contact. A simplified version of our data looks like this:

```rust
#[derive(Clone, Data)]
struct Contact {
    // fields
}

type ContactId = u64;

#[derive(Clone, Data)]
struct Contacts {
    inner: Arc<HashMap<ContactId, Contact>>,
}

// Lets write a lens that returns a specific contact based on its id, if it exists.

struct ContactIdLens(ContactId);

impl Lens<Contacts, Option<Contact>> for ContactIdLens {
    fn with<R, F: FnOnce(&Option<Contact>) -> R>(&self, data: &Contacts, f: F) -> R {
        let contact = data.inner.get(&self.0).cloned();
        f(&contact)
    }

    fn with_mut<R, F: FnOnce(&mut Option<Contact>) -> R>(&self, data: &mut Contacts, f: F) -> R {
        // get an immutable copy
        let mut contact = data.inner.get(&self.0).cloned();
        let result = f(&mut contact);
        // only actually mutate the collection if our result is mutated;
        if !data.inner.get(&self.0).same(&contact.as_ref()) {
            let contacts = Arc::make_mut(&mut data.inner);
            // if we're none, we were deleted, and remove from the map; else replace
            match contact {
                Some(contact) => contacts.insert(self.0, contact),
                None => {
                    contacts.remove(self.0);
                }
            }
        }
        result
    }
}
```

### doing a conversion

What if you have a distance in miles that you would like to display in
kilometres?


```rust
struct MilesToKm;

const KM_PER_MILE: f64 = 1.609344;

impl Lens<f64, f64> for MilesToKm {
    fn with<R, F: FnOnce(&f64) -> R>(&self, data: &f64, f: F) -> R {
        let kms = *data * KM_PER_MILE;
        f(&kms)
    }

    fn with_mut<R, F: FnOnce(&mut f64) -> R>(&self, data: &mut f64, f: F) -> R {
        let mut kms = *data * KM_PER_MILE;
        let kms_2 = kms;
        let result = f(&mut kms);
        // avoid doing the conversion if unchanged, it might be lossy?
        if !kms.same(&kms_2) {
            let miles = kms * KM_PER_MILE.recip();
            *data = miles;
        }
        result
    }
}
```
