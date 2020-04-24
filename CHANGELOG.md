# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `TextBox` can receive `EditAction` commands
  ([#814](https://github.com/xi-editor/druid/pull/814) by [@cmyr])

### Changed

- `Split` constructors are now `Split::rows` and `columns`.
  `Split` is more accurate as well.
  ([#738](https://github.com/xi-editor/druid/pull/738) by [@xStorm])

### Deprecated

### Removed

### Fixed

- Reduce the flashing in ext_event and identity examples.
  ([#782](https://github.com/xi-editor/druid/pull/782) by [@futurepaul])

- GTK uses the system locale.
  ([#798](https://github.com/xi-editor/druid/pull/798) by [@finnerale])

- GTK windows get actually closed.
  ([#797](https://github.com/xi-editor/druid/pull/797) by [@finnerale])

### Docs

- `Env` got a new example and usage hints.
  ([#796](https://github.com/xi-editor/druid/pull/796) by [@finnerale])

### Cleanups

- Replace `#[macro_use]` with normal `use`.
  ([#808](https://github.com/xi-editor/druid/pull/808) by [@totsteps])

### Outside News

- A new project using druid: [Kondo](https://github.com/tbillington/kondo) Save disk space by cleaning unneeded files from software projects.

## [0.5] - 2020-04-01

Last release without a changelog :(


[@xStorm]: https://github.com/xStorm

[@cmyr]: https://github.com/cmyr

[@totsteps]: https://github.com/totsteps

[@finnerale]: https://github.com/finnerale

[@futurepaul]: https://github.com/futurepaul



[Unreleased]: https://github.com/xi-editor/druid/compare/v0.5.0...master

[0.5]: https://github.com/xi-editor/druid/compare/v0.4.0...v0.5.0


