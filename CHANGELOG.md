# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Highlights

#### Basic X11 backend for druid-shell. ([#599])

[@crsaracco] has implemented basic support to run druid on bare-metal X11!

While still incomplete this lays the foundation for running druid on Linux without relying on GTK.

### Added

- `TextBox` can receive `EditAction` commands. ([#814] by [@cmyr])

- Added `Split::min_splitter_area(f64)` to add padding around the split bar. ([#738] by [@xStorm])

- The `druid::text` module is public now. ([#816] by [@cmyr])

- Basic X11 backend for druid-shell. ([#599] by [@crsaracco])

### Changed

- `Split` constructors are now called `Split::rows` and `columns`. ([#738] by [@xStorm])

- `Split::splitter_size` no longer includes padding. ([#738] by [@xStorm])

- `Event::MouseMoved` has been renamed to `MouseMove`. ([#825] by [@teddemunnik])

- `has_focus` no longer returns false positives. ([#819] by [@xStorm])

### Deprecated

### Removed

- The optional GTK feature for non-Linux platforms. ([#611] by [@pyroxymat])

### Fixed

- Reduce the flashing in ext_event and identity examples. ([#782] by [@futurepaul])

- GTK uses the system locale. ([#798] by [@finnerale])

- GTK windows get actually closed. ([#797] by [@finnerale])

- Windows now respects the minimum window size. ([#727] by [@teddemunnik])

- Windows now respects resizability. ([#712] by [@teddemunnik])

- Mouse capturing on Windows. ([#695] by [@teddemunnik])

- Focus cycling now works even starting from non-registered-for-focus widgets. ([#819] by [@xStorm])

- `Event::FocusChanged` gets propagated to focus gaining widgets. ([#819] by [@xStorm])

### Visual
- Improved `Split` accuracy. ([#738] by [@xStorm])

### Docs

- `Env` got a new example and usage hints. ([#796] by [@finnerale])

- Usage of bloom filters got documented. ([#818] by [@xStorm])

### Cleanups

- Replace `#[macro_use]` with normal `use`. ([#808] by [@totsteps])

### Outside News

- A new project using druid: [Kondo](https://github.com/tbillington/kondo) Save disk space by cleaning unneeded files from software projects.

[#819]: https://github.com/xi-editor/druid/pull/819
[#599]: https://github.com/xi-editor/druid/pull/599
[#611]: https://github.com/xi-editor/druid/pull/611
[#695]: https://github.com/xi-editor/druid/pull/695
[#712]: https://github.com/xi-editor/druid/pull/712
[#727]: https://github.com/xi-editor/druid/pull/727
[#738]: https://github.com/xi-editor/druid/pull/738
[#782]: https://github.com/xi-editor/druid/pull/782
[#796]: https://github.com/xi-editor/druid/pull/796
[#797]: https://github.com/xi-editor/druid/pull/797
[#798]: https://github.com/xi-editor/druid/pull/798
[#808]: https://github.com/xi-editor/druid/pull/808
[#814]: https://github.com/xi-editor/druid/pull/814
[#816]: https://github.com/xi-editor/druid/pull/816
[#818]: https://github.com/xi-editor/druid/pull/818
[#825]: https://github.com/xi-editor/druid/pull/825

## [0.5] - 2020-04-01

Last release without a changelog :(


[@pyroxymat]: https://github.com/pyroxymat

[@crsaracco]: https://github.com/crsaracco

[@teddemunnik]: https://github.com/teddemunnik

[@xStorm]: https://github.com/xStorm

[@cmyr]: https://github.com/cmyr

[@totsteps]: https://github.com/totsteps

[@finnerale]: https://github.com/finnerale

[@futurepaul]: https://github.com/futurepaul



[Unreleased]: https://github.com/xi-editor/druid/compare/v0.5.0...master

[0.5]: https://github.com/xi-editor/druid/compare/v0.4.0...v0.5.0


