# Druid

Druid is a framework for building simple graphical applications.

Druid is composed of a number of related projects. [`druid-shell`] is a
low-level library that provides a common abstraction for interacting with the
current OS & window manager. [`piet`] is an abstraction for doing 2D graphics;
[`kurbo`] is a library for 2D geometry; and [`druid`] itself is an opinionated set of
high-level APIs for building cross-platform desktop applications.

Druid is *data oriented*. It shares many ideas (and is directly inspired by)
contemporary declarative UI frameworks such as [Flutter], [Jetpack Compose],
and [SwiftUI], while also attempting to be conceptually simple and largely
*non-magical*. A programmer familiar with Rust should be able to understand how
Druid works without special difficulty.

## Goals and Status

The current goal of Druid is to make it easy to write a program in Rust that
can present a GUI and accept user input. Running your program should be as
simple as `cargo run`.

## Key Concepts

- **[the `Data` trait]**: How you represent your application model.
- **[the `Widget` trait]**: How you represent your UI.
- **[the `Lens` trait]**: How you associate parts of your model with parts of
  your UI.


[`druid-shell`]: https://docs.rs/druid-shell
[`druid`]: https://docs.rs/druid
[`piet`]: https://docs.rs/piet
[`kurbo`]: https://docs.rs/kurbo
[Flutter]: https://flutter.dev
[Jetpack Compose]: https://developer.android.com/jetpack/compose
[SwiftUI]: https://developer.apple.com/documentation/swiftui
[the `Data` trait]: ./data.md
[the `Widget` trait]: ./widget.md
[the `Lens` trait]: ./lens.md
