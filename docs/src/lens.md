# Lenses and the `Lens` trait

One of the key abstractions in `druid` along with `Data` is the `Lens` trait. This page explains what they are, and then how to use them. `Lens`es may seem complicated at first, but they are also very powerful, allowing you to write code that is reusable, concise, and understandable (once you understand `Lens`es themselves).

## Fundamentals: Definition and Implementation

Like Rust itself, lenses are one of those things that require effort up front to learn, but are very fun and effective to use once you understand them. This section represents the effort part of the equation. I promise if you stick with it you will reap the rewards.

### Definition

Let's start with the definition of a `Lens`:

```rust
pub trait Lens<T, U> {
    fn with<F: FnOnce(&U)>(&self, data: &T, f: F);

    fn with_mut<F: FnOnce(&mut U)>(&self, data: &mut T, f: F);
}
```

I've copied this definition from the `druid` source code, but then simplified it a little, by removing the return types, as they are not fundamental to the way lenses work.

The first thing to notice is the generics on the `Lens` itself. There are 3 types involve in the lens: the lens itself, `T` and `U`. The two type parameters represent the mis-match that lenses solve: we have a function that operates on `U`, and an object of type `T`, so we need to transform `T` into `U` somehow.

### Implementation

Time for an example. Let's implement & use `Lens` manually so we can see what's going on.

```rust
struct Container {
    inner: String,
    another: String,
}

// Here the lens doesn't have any data, but there are cases where
// it might, for example it might contain an index into a collection.
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

This is a very simple case. All we need to do is project the function onto the field. Notice that this isn't the only valid lens from `Container` to `String` we could have made - we could also project from `Container` to `another`. We made the choice how to transform `Container` into `String` when we implemented `Lens`.

> Side note: Actually we could project on to any string we have access to, including something in a global mutex, or a string that we create and discard in the lens. Lenses made like this are usually not what you want.

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

The actual definition of `Lens` in `druid` allows the user to return values from the lens. This isn't necessary for the core functioning of the lens, but it is useful. Also, because the types `T` and `U` always appear behind pointers (`&` and `&mut`), we can relax the `Sized` requirement that is applied by default, meaning we can implement `Lens` for types like `[T]` (slice) and `str`.

Here is the real definition for completeness:

```rust
pub trait Lens<T: ?Sized, U: ?Sized> {
    fn with<V, F: FnOnce(&U) -> V>(&self, data: &T, f: F) -> V;

    fn with_mut<V, F: FnOnce(&mut U) -> V>(&self, data: &mut T, f: F) -> V;
}
```

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

Let's look at the code that get's generated (I captured this using `cargo-expand`, then removed some unimportant bits).

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

If I told you that the concept of lenses comes from Haskell (the functional megolith), I'm sure you won't be suprised when I also tell you that they really excel when it comes to composition. Let's say we have an outer struct that contains an inner struct, with the inner struct containing a `String`. Now let's say we want to tell a label widget to display the string as text in a label. We could write a lens from the outer struct to the string, which would look something like `f(&outer.inner.text)`, but actually we don't need to do this: we can use the `then` combinator. The full example is below

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

> Side note: Because unlike Haskell, Rust has all the type information during monomorphisation, it can inline all these functions and the extra cost of having 2 lens functions disappears, leaving what you would have written by hand. When you're waiting for Rust's infamously long compilations to finish, know that you're waiting for a potentially significant run-time benefit in exchange.

`LensExt` contains a few more useful methods for handling things like negating a boolean, or auto-`Deref`ing a value.

There are also 3 special structs in `druid::lens`: `Constant`, `Identity` and `Unit`. `Constant` is a lens that always returns the same value, and always discards any changes, while `Identity` is a lens that does nothing. You might say "what is the point of a lens that does nothing", which would be a fair question. Well, there are some places where a lens is required, and having an identity allows the user to say act as if there was no lens. It's also used to begin a composition chain using the combinators like `then`. `Unit` is a special case of `Constant` where the constant in question is `()`.

> Side note: Because `()` only has 1 value, it does actually respect mutations. It's just that mutations always result in the same value again (`()`).

### The `lens` macro

Finally, there is a macro for constructing lenses on the fly. It allows you to lens into fields of a struct you don't control (so you can't derive `Lens` for it), it also allows lensing into tuples and tuple structs, and lastly it will create index lenses into slices.

### Wrapping up

Whew, that was quite complicated. Hopefully now you have a solid understanding of the problem that lenses solve, how they solve it, and how to use them effectively. Now you have the ability to relate your application data to widget data, allowing you to use reusable widgets in any configuration you want. If any parts of this page are confusing, please open an issue on the issue tracker or mention it on zulip, and we will see if we can improve the docs (and clear up any misunderstandings you might have).

### Bonus - mapping between complex structs automatically

Way back in the first section, we discussed lenses between the following:

```rust
#[derive(Lens)]
struct Container {
    first_name: Rc<str>,
    last_name: Rc<str>,
    age: u16,
}

struct Name {
    first: Rc<str>,
    last: Rc<str>,
}
```

We showed that you can construct a lens from `Container` to `Name`, but it was a bit involved and required knowledge of the inner workings of `Lens`, something you probably don't want to think about. The crate `druid-lens-compose` is an experiment to allow for building a lens to a struct out of lenses to its fields. It's not well documented, but usage is fairly simple: derive the macro for the inner struct you want to lens to, run `cargo doc`, and look at the signature of the generated method/build struct. We'd be interested to hear if you found it useful, so please drop by the zulip and let us know!
