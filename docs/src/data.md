# Model data and the `Data` trait

The heart of a `druid` application is your application model. Your model drives
your UI. When you mutate your model, druid compares the old and new version,
and propagates the change to the components ('widgets') of your application that
are affected by the change.

For this to work, your model must implement the `Clone` and `Data` traits. It
is important that your model be cheap to clone; we encourage the use of
reference counted pointers to allow cheap cloning of more expensive types. `Arc`
and `Rc` have blanket `Data` impls, so if you have a type that does not
implement `Data`, you can always just wrap it in one of those smart pointers.

The `Data` trait has a single method:

```rust,no_run,noplaypen
{{#include ../../druid/src/data.rs:same_fn}}
```

#### Derive

`Data` can be derived. This is recursive; it requires `Data` to be implemented
for all members. For 'C style' enums (enums where no variant has any fields)
this also requires an implementation of `PartialEq`. `Data` is implemented for
a number of `std` types, including all primitive types, `String`, `Arc`, `Rc`,
as well as `Option`, `Result`, and various tuples whose members implement
`Data`.

Here is an example of using `Data` to implement a simple data model.

```rust
{{#include ../book_examples/src/data_md.rs:derive}}
```
