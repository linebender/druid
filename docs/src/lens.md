# Lenses and the `Lens` trait

One of the key abstractions in `druid` along with `Data` is the `Lens` trait. This page explains what they are, and then how to use them. `Lens`es may seem complicated at first, but they are also very powerful, allowing you to write code that is reusable and concise, understandable and powerful (once you understand `Lens`es themselves).

## Fundamentals: Definition and Implementation

Like Rust itself, lenses are one of those things that require effort up front to learn, but are very fun and powerful to use once you understand them. This section represents the effort part of the equation. I promise if you stick with it you will reap the rewards.

Let's start with the definition of a `Lens`:

```rust
pub trait Lens<T, U> {
    fn with<F: FnOnce(&U)>(&self, data: &T, f: F);

    fn with_mut<F: FnOnce(&mut U)>(&self, data: &mut T, f: F);
}
```

I've copied this definition from the `druid` source code, but then simplified it a little, by removing the return types, as they are not fundamental to the way lenses work.

The first thing to notice is the generics on the `Lens` itself. There are 3 types involve in the lens: the lens itself, `T` and `U`. The two type parameters represent the mis-match that lenses solve: we have a function that operates on `U`, and an object of type `T`, so we need to transform `T` into `U` somehow.

Time for an example. Let's implement & use `Lens` manually so we can see what's going on.

```rust
struct Container {
    inner: String,
    another: String,
}

// Here the lens doesn't have any data, but there are cases where
// it might, for example it might contain an index into a collection.
struct InnerLens;

// Our lens will apply functions that take a `u8` to a `Container`.
impl Lens<Container, String> for InnerLens {
    fn with<F: FnOnce(&String)>(&self, data: &Container, f: F) {
        f(&data.inner);
    }

    fn with_mut<F: FnOnce(&mut String)>(&self, data: &mut Container, f: F) {
        f(&mut data.inner);
    }
}
```

This is a very simple case. All we need to do is project the function onto the field. Notice that this isn't the only vaid lens from `Container` to `String` we could have made - we could also project from `Container` to `another`. We made the choice how to transform `Container` into `String` when we implemented `Lens`.

You'll also notice that both methods take an immutable reference to `self`, even the `mut` variant. The lense itself should be thought of as a fixed thing that knows how to do the mapping. In the above case it contains no data, and will most likely not even be present in the final compiled/optimized code.

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
    fn with<F: FnOnce(&Name)>(&self, data: &Container, f: F) {
        let first = data.first_name.clone();
        let last = data.last_name.clone();
        f(&Name { first, last });
    }

    fn with_mut<F: FnOnce(&mut Name)>(&self, data: &mut Container, f: F) {
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

Now as I'm sure you've realised, the above is very inefficient. Given that we will be traversing our data very often, we need it to be cheap. (This wasn't a problem before, because when we don't need to build the inner type, we can just use references. It also wouldn't be a problem if our data was cheap to copy/clone, for example any of the primitive number types `u8`, ... `f64`) Luckily, this is exactly the kind of thing that rust excels at. Let's rewrite the above example to be fast!

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

The trade-off is that we introduce more complexity into the `Name` type: to make changes to the data we have to use `Rc::make_mut` to get mutable access to the `String`. (The code in the lens will ensure that the newer copy of the `Rc`d data is saved to the outer type.) This means the writing fast druid code requires knowledge of the Rust pointer types (`Rc`/`Arc`, and also potentially `RefCell`/`Mutex`).

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

Now in addition to almost free `Clone`s, we also have cheap incremental updates to the data itself. In the case of names, this isn't that important, but if the vector had `1_000_000_000` elements, we could still make changes in only *O(log(n))* time (in this case the difference between `1_000_000_000` and `30` - pretty big!).

Right, now you understand how `Lens`es work. Congratulations, you've done the hardest bit! If you get lost later on, read this section again, and eventually it will all make sense.

### Bonus - The actual `Lens` definition

The actual definition of `Lens` in `druid` allows the user to return values from the lens. This isn't strictly necessary, one could always use a value captured in the closure to store the result, but it is useful. Also, because the types `T` and `U` always appear behind pointers (`&` and `&mut`), we can relax the `Sized` requirement that is applied by default, meaning we can implement `Lens` for types like `[T]` (slice) and `str`.

Here is the real definition for completeness:

```rust
pub trait Lens<T: ?Sized, U: ?Sized> {
    fn with<V, F: FnOnce(&U) -> V>(&self, data: &T, f: F) -> V;

    fn with_mut<V, F: FnOnce(&mut U) -> V>(&self, data: &mut T, f: F) -> V;
}
```

## Lenses in Druid

Now on to the more fun bit: how we can use `Lens`es to get all those lovely qualities we talked about in the introduction. What you'll notice in this section is that we rarely have to build lenses ourself: we can often get what we want using the `Lens` proc macro, or through the functions in `LensExt`.

Let's go back to the first example we looked at, with one of the fields removed for simplicity:

```rust
#[derive(Lens)]
struct Container {
    inner: u8,
}
```

Let's look at the code that get's generated (I captured this using `cargo-expand`).



