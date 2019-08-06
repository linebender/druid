# druid

## Data-oriented Rust User Interface Design toolkit

Druid is a new Rust-native UI toolkit, still in early stages. Its main
goal is performance, also aiming for small binary size and compile time,
fast startup, and very easy build configuration (just `cargo run`). 
It currently supports Windows and macOS, with GNU/Linux support planned.

Raph gave a talk at the July 2018 SF Rust Meetup ([video][jul-2018-video],
[slides][jul-2018-slides]) about the design. Traditional object-oriented
designs have proved to be cumbersome.

The layout protocol is inspired by Flutter. It is originally being developed
primarily for use by [xi-win], but has the potential to be useful for other
applications. Due to its focus on performance and the relative paucity of
toolkit-provided components, it is more likely to be useful for games and
music software than general-purpose GUI applications. It is currently used
as the basis for [Synthesizer IO].

### The druid-win-shell layer

The druid-win-shell layer is primarily an abstraction for displaying a window
and receiving events. As such, it is an alternative to [winit].

Ideally all Windows-specific logic (including all uses of `unsafe`) are isolated
to the druid-win-shell subcrate, and druid proper is cross platform and with
no unsafe code. That is not entirely the case now.

## Evolution

This crate is currently in early stages. More features will be built out as
needed for use in the flagship apps, and this will inevitably lead to API
instability. Features hoped-for soon include:

  - [ ] More of the basic widgets
  - [ ] Incremental presentation
  - [ ] Some simple theming support

The biggest single obstacle to porting is 2d graphics, as druid currently
uses Direct2D (and DirectWrite for text). One way forward is to create a
[2d graphics] abstraction.

## Alternatives

In addition to wrappers for mature UI toolkits (mostly C++), [conrod]
and [azul] are interesting Rust-native efforts with promise to become usable.

With a focus on 2D games, [ggez] also looks promising.

## Contributions

We gladly accept contributions via GitHub pull requests. Please see [CONTRIBUTING.md] for more details.

A very good place to ask questions and discuss development work is our
[Zulip chat instance], in the #druid channel.

## Authors

The main author is Raph Levien.

[xi-win]: https://github.com/xi-editor/xi-win
[winit]: https://github.com/tomaka/winit
[Synthesizer IO]: https://github.com/raphlinus/synthesizer-io
[jul-2018-video]: https://www.youtube.com/watch?v=4YTfxresvS8
[jul-2018-slides]: https://docs.google.com/presentation/d/1aDTRl5R-icAF38Di-qJ4FzAl3pLlutTKVFcr3mUGgYo/edit?usp=sharing
[2d graphics]: https://raphlinus.github.io/rust/graphics/2018/10/11/2d-graphics.html
[conrod]: https://github.com/PistonDevelopers/conrod
[azul]: https://github.com/maps4print/azul
[ggez]: https://github.com/ggez/ggez
[CONTRIBUTING.md]: CONTRIBUTING.md
[Zulip chat instance]: https://xi.zulipchat.com
