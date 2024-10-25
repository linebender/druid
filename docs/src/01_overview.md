# Druid

**UNMAINTAINED**

**The Druid project has been discontinued.**

New development effort moved on to [Xilem], which has a lot of fundamental changes to allow for
a wider variety of applications with better performance, but it also heavily inherits from Druid.
We see [Xilem] as the future of Druid.

## Introduction

Druid is a framework for building simple graphical applications.

Druid is composed of a number of related projects. [`druid-shell`] is a
low-level library that provides a common abstraction for interacting with the
current OS & window manager. [`piet`] is an abstraction for doing 2D graphics;
[`kurbo`] is a library for 2D geometry; and [`druid`] itself is an opinionated set of
high-level APIs for building cross-platform desktop applications.

The framework is *data oriented*. It shares many ideas (and is directly inspired by)
contemporary declarative UI frameworks such as [Flutter], [Jetpack Compose],
and [SwiftUI], while also attempting to be conceptually simple and largely
*non-magical*. A programmer familiar with Rust should be able to understand how
Druid works without special difficulty.

## Prerequisites

This tutorial assumes basic familiarity with Rust and a working setup with the basic tooling like
Rustup and Cargo. This tutorial will use stable Rust (v1.65.0 at the time of writing) and the latest
released version of Druid (v0.8).

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
[the `Data` trait]: ./03_data.md
[the `Widget` trait]: ./04_widget.md
[the `Lens` trait]: ./05_lens.md
[Xilem]: https://github.com/linebender/xilem
