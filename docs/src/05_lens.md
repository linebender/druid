# Lenses and the `Lens` trait

One of the key abstractions in `druid` along with `Data` is the `Lens` trait. This page explains what they are, and then how to use them. `Lens`es are a complex but powerful concept, that allow you to abstract over the notion of "X owns an instance of Y".


## Fundamentals: Definition and Implementation

### Definition

Let's start with the (simplified) definition of a `Lens`:

```rust
pub trait Lens<T, U> {
    fn with<F: FnOnce(&U)>(&self, data: &T, f: F);

    fn with_mut<F: FnOnce(&mut U)>(&self, data: &mut T, f: F);
}
```

The first thing to notice is the generics on the `Lens` itself. There are 3 types involved in the lens: `Self` (the lens itself), `T` and `U`. The two type parameters represent the mismatch that lenses solve: we have a function that operates on `U`, and an object of type `T`, so we need to transform `T` into `U` somehow.

### Implementation

As an example, let's write a manual implementation of the `Lens` trait:

```rust
struct Container {
    inner: String,
    another: String,
}

// This lens doesn't have any data, because it will always map to the same field.
// A lens that mapped to, say, an index in a collection, would need to store that index.
struct InnerLens;

// Our lens will apply functions that operate on a `String` to a `Container`.
impl Lens<Container, String> for InnerLens {
    fn with<F: FnOnce(&String)>(&self, data: &Container, f: F) {
        f(&data.inner);
    }

    fn with_mut<F: FnOnce(&mut String)>(&self, data: &mut Container, f: F) {
        f(&mut data.inner);
    }
}
```

The implementation is straightforward: it projects the given function onto the `inner` field of our struct. (Notice that this isn't the only valid lens from `Container` to `String` we could have made - we could also project from `Container` to `another`).

You'll also notice that both methods take an immutable reference to `self`, even the `mut` variant. The lense itself should be thought of as a fixed value that knows how to do the mapping. In the above case it contains no data, and will likely not even be present in the final compiled/optimized code.

Now for a slightly more involved example

```rust
struct Container2 {
    first_name: String,
    last_name: String,
    age: u16, // in the future maybe people will live past 256?
}

struct Name {
    first: String,
    last: String,
}

struct NameLens;

impl Lens<Container2, Name> for NameLens {
    fn with<F: FnOnce(&Name)>(&self, data: &Container2, f: F) {
        let first = data.first_name.clone();
        let last = data.last_name.clone();
        f(&Name { first, last });
    }

    fn with_mut<F: FnOnce(&mut Name)>(&self, data: &mut Container2, f: F) {
        let first = data.first_name.clone();
        let last = data.last_name.clone();
        let mut name = Name { first, last };
        f(&mut name);
        data.first_name = name.first;
        data.last_name = name.last;
    }
}
```

> Side note: if you try doing this with `struct Name<'a> { first: &'a String, ...`, you'll find that it's not possible to be generic over the mutability of the fields in `Name`, so we can't make the `Name` struct borrow the data both mutably and immutably. Even if we could in this case, things quickly get very complicated. Also, sometimes `Widget`s need to keep a copy of the data around for use internally. For now the accepted best practice is to make `Clone`ing cheap and use that.

Now as I'm sure you've realised, the above is very inefficient. Given that we will be traversing our data very often, we need it to be cheap. (This wasn't a problem before, because when we don't need to build the inner type, we can just use references. It also wouldn't be a problem if our data was cheap to copy/clone, for example any of the primitive number types `u8`, ... `f64`.) Luckily, this is exactly the kind of thing that rust excels at. Let's rewrite the above example to be fast!

```rust
struct Container2 {
    first_name: Rc<String>,
    last_name: Rc<String>,
    age: u16,
}

struct Name {
    first: Rc<String>,
    last: Rc<String>,
}

struct NameLens;

impl Lens<Container2, Name> for NameLens {
    // .. identical to previous example
}
```

As you'll see, we've introduced `Rc`: the reference-counted pointer. You will see this and its multithreaded cousin `Arc` used pervasively in the examples. Now, the only time we actually have to copy memory is when `Rc::make_mut` is called in the `f` in `with_mut`. This means that in the case where nothing changes, all we will be doing is incrementing and decrementing reference counts. Moreover, we give the compiler the opportunity to inline `f` and `with`/`with_mut`, making this abstraction potentially zero-cost (disclaimer: I haven't actually studied the produced assembly to validate this claim).

The trade-off is that we introduce more complexity into the `Name` type: to make changes to the data we have to use `Rc::make_mut` to get mutable access to the `String`. (The code in the lens will ensure that the newer copy of the `Rc`d data is saved to the outer type.) This means the writing fast Druid code requires knowledge of the Rust pointer types (`Rc`/`Arc`, and also potentially `RefCell`/`Mutex`).

We can actually do even better than this. Suppose that we are working on a vector of data rather than a string. We can import the `im` crate to get collections that use *structural sharing*, meaning that even when the vector is mutated, we only *Clone* what we need to. Because `im` is so useful, it is included in `druid` (behind the `im` feature).

```rust
struct Container2 {
    // Pretend that it's the 1980s and we store only ASCII names.
    first_name: im::Vector<u8>,
    last_name: im::Vector<u8>,
    age: u16,
}

struct Name {
    first: im::Vector<u8>,
    last: im::Vector<u8>,
}

struct NameLens;

impl Lens<Container2, Name> for NameLens {
    // .. identical to previous example
}
```

Now in addition to almost free `Clone`s, we also have cheap incremental updates to the data itself. That means your UI won't get slowdowns if your data structure gets very large (eg a list of entries in a database).

Hopefully, this makes sense to you. This was a technical overview of lenses as generic data structures. The next section will cover how lenses are integrated in Druid in more detail.


## Lenses in Druid

Now on to the more fun bit: how we can use `Lens`es to get all those lovely qualities we talked about in the introduction. What you'll notice in this section is that we rarely have to build lenses ourself: we can often get what we want using the `Lens` proc macro, or through the functions in `LensExt`.

### Deriving lenses

Let's go back to the first example we looked at, with one of the fields removed for simplicity:

```rust
#[derive(Lens)]
struct Container {
    inner: u8,
}
```

Let's look at the code that gets generated (I captured this using `cargo-expand`, then removed some unimportant bits).

```rust
pub mod container_derived_lenses {
    #[allow(non_camel_case_types)]
    pub struct inner;
}
impl druid::Lens<Container, u8> for container_derived_lenses::inner {
    fn with<V, F: FnOnce(&u8) -> V>(&self, data: &Container, f: F) -> V {
        f(&data.inner)
    }
    fn with_mut<V, F: FnOnce(&mut u8) -> V>(&self, data: &mut Container, f: F) -> V {
        f(&mut data.inner)
    }
}
#[allow(non_upper_case_globals)]
impl Container {
    pub const inner: container_derived_lenses::inner = container_derived_lenses::inner;
}
```

The macro has created a new module with a long name, put a struct in it that breaks the type naming convention, implemented `Lens` on the type, and then put a constant in an `impl` block for your data type with the same name. The upshot is that we can do `StructName::field_name` and get a lens from the struct to its field.

> Side note: Doing this makes using the lenses very simple (you just do `StructName::field_name`), but it can be a bit confusing, because of breaking the naming conventions. This is the reason I've included the expanded code in the page.

### Composing lenses

If I told you that the concept of lenses comes from Haskell (the functional megolith), I'm sure you won't be surprised when I also tell you that they really excel when it comes to composition. Let's say we have an outer struct that contains an inner struct, with the inner struct containing a `String`. Now let's say we want to tell a label widget to display the string as text in a label. We could write a lens from the outer struct to the string, which would look something like `f(&outer.inner.text)`, but actually we don't need to do this: we can use the `then` combinator. The full example is below

```rust
#[derive(Lens)]
struct Outer {
    inner: Inner,
}

#[derive(Lens)]
struct Inner {
    text: String
}

// `composed_lens` will contain a lens that goes from `Outer` through `Inner` to `text`.
let composed_lens = Outer::inner.then(Inner::text);
```

`LensExt` contains a few more useful methods for handling things like negating a boolean, or auto-`Deref`ing a value.

There are also 3 special structs in `druid::lens`: `Constant`, `Identity` and `Unit`. `Constant` is a lens that always returns the same value, and always discards any changes, while `Identity` is a lens that does nothing. You might say "what is the point of a lens that does nothing", which would be a fair question. Well, there are some places where a lens is required, and having an identity allows the user to say act as if there was no lens. It's also used to begin a composition chain using the combinators like `then`. `Unit` is a special case of `Constant` where the constant in question is `()`.

### The `lens` macro

Finally, there is a macro for constructing lenses on the fly. It allows you to lens into fields of a struct you don't control (so you can't derive `Lens` for it), it also allows lensing into tuples and tuple structs, and lastly it will create index lenses into slices.

### Wrapping up

Whew, that was quite complicated. Hopefully now you have a solid understanding of the problem that lenses solve, how they solve it, and how to use them effectively.

If any parts of this page are confusing, please open an issue on the issue tracker or mention it on zulip, and we will see if we can improve the docs (and clear up any misunderstandings you might have).
