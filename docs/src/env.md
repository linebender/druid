# The `Env`

The [`Env`] represents the environment; it is intended as a way of managing
and accessing state about your specific application, such as color schemes,
localized strings, and other resources.

The `Env` is created when the application is launched, and is passed down to all
widgets. The `Env` may be modified at various points in the tree; values in the
environment can be overridden with other values of the same type, but they can
never be removed. If something exists in the `Env` at a given level of the tree,
it will exist for everything 'below' that level; that is, for all children of that
widget.

## `Key`s, `Value`s, and themes

The most prominent role of `Env` is to store a set of typed keys and values. The
`Env` can only store a few types of things; these are represented by the
[`Value`] type, which looks like this:

```rust,noplaypen
{{#include ../../druid/src/env.rs:value_type}}
```

The only way to get an item out of the `Env` is with a [`Key`]. A [`Key`] is
a combination of a string identifier and a type.

You can think of this as strict types, enforced at runtime. This is less scary
than it sounds, assuming the user follows a few simple guidelines. That said, **It is the
programmer's responsibility to ensure that the environment is used correctly**.
The API is aggressive about checking for misuse, and many methods will panic if
anything is amiss. In practice this should be easy to avoid, by following a few
simple guidelines.

1. **`Key`s should be `const`s with unique names.** If you need to use a custom
   key, you should declare it as a `const`, and give it a unique name. By
   convention, you should namespace your keys using something like [reverse-DNS]
   notation, or even just prefixing them with the name of your app.

    ```rust,noplaypen
    const BAD_NAME: Key<f64> = Key::new("height");
    const GOOD_NAME: Key<f64> = Key::new("com.example.my-app.main-view-height");
    ```
1. **`Key`s must always be set before they are used.** In practice this means
   that most keys are set when your application launches, using
   [`AppLauncher::configure_env`]. Once a key has been added to the `Env`, it
   cannot be deleted, although it can be overwritten.

1. **Values can only be overwritten by values of the same type.** If you have a
   `Key<f64>`, assuming that key has already been added to the `Env`, you cannot
   replace it with any other type.

Assuming these rules are followed, `Env` should just work.

### KeyOrValue

Druid includes a [`KeyOrValue`] type that is used for setting certain properties
of widgets. This is a type that can be *either* a concrete instance of some
type, *or* a `Key` that can be used to get that type from the `Env`.

```rust,noplaypen
{{#include ../book_examples/src/env_md.rs:key_or_value}}
```

### EnvScope

You may override values in the environment for a given widget (and all of its
children) by using the [`EnvScope`] widget. This is easiest when combined with
the [`env_scope`] method on [`WidgetExt`]:

```rust,noplaypen
{{#include ../book_examples/src/env_md.rs:env_scope}}
```


## Localization

*localization is currently half-baked*

The `Env` contains the localization resources for the current locale. A
[`LocalizedString`] can be resolved to a given string in the current locale by
calling its [`resolve`] method.

In general, you should not need to worry about localization directly. See the
[localization] chapter for an overview of localization in Druid.


[`Env`]: https://docs.rs/druid/0.6.0/druid/struct.Env.html
[`Key`]: https://docs.rs/druid/0.6.0/druid/struct.Key.html
[`Value`]: https://docs.rs/druid/0.6.0/druid/struct.Value.html
[`LocalizedString`]: https://docs.rs/druid/0.6.0/druid/struct.LocalizedString.html
[`resolve`]: https://docs.rs/druid/0.6.0/druid/struct.LocalizedString.html#method.resolve
[localization]: ./localization.md
[reverse-DNS]: https://en.wikipedia.org/wiki/Reverse_domain_name_notation
[`AppLauncher::configure_env`]: https://docs.rs/druid/0.6.0/druid/struct.AppLauncher.html#method.configure_env
[`KeyOrValue`]: https://docs.rs/druid/0.6.0/druid/enum.KeyOrValue.html
[`EnvScope`]: https://docs.rs/druid/0.6.0/druid/widget/struct.EnvScope.html
[`WidgetExt`]: https://docs.rs/druid/0.6.0/druid/trait.WidgetExt.html
[`env_scope`]: https://docs.rs/druid/0.6.0/druid/trait.WidgetExt.html#method.env_scope
