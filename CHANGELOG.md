# Changelog

The latest published druid release is [0.6.0](#060---2020-06-01) which was released on 2020-06-01.
You can find its changes [documented below](#060---2020-06-01).

## [Unreleased]

### Highlights

### Added

### Changed

- `Image` and `ImageData` exported by default. ([#1011] by [@covercash2])

### Deprecated

### Removed

### Fixed

### Visual

- `TextBox` stroke remains inside its `paint_rect`. ([#1007] by [@jneem])

### Docs

- Fixed a link in `druid::command` documentation. ([#1008] by [@covercash2])

### Examples

### Maintenance

- Standardized web targeting terminology. ([#1013] by [@xStrom])

### Outside News

## [0.6.0] - 2020-06-01

### Highlights

#### X11 backend for druid-shell.

[@crsaracco] got us started and implemented basic support to run druid on bare-metal X11 in [#599].
Additional features got fleshed out in [#894] and [#900] by [@xStrom]
and in [#920], [#961], and [#982] by [@jneem].

While still incomplete this lays the foundation for running druid on Linux without relying on GTK.

#### Web backend for druid-shell.

[@elrnv] continued the work of [@tedsta] and implemented a mostly complete web backend
via WebAssembly (Wasm) in [#759] and enabled all druid examples to
[run in the browser](https://elrnv.github.io/druid-wasm-examples/).

While some features like the clipboard, menus or file dialogs are not yet available,
all fundamental features are there.

#### Using Core Graphics on macOS.

[@cmyr] continued the work of [@jrmuizel] and implemented Core Graphics support for piet in
[piet#176](https://github.com/linebender/piet/pull/176).

Those changes made it into druid via [#905].
This means that druid no longer requires cairo on macOS and uses Core Graphics instead.

### Added

- Standardized and exposed more methods on more contexts. ([#970], [#972], [#855] by [@cmyr], [#898] by [@finnerale], [#954] by [@xStrom], [#917] by [@jneem])
- `im` feature, with `Data` support for the [`im` crate](https://docs.rs/im/) collections. ([#924] by [@cmyr])
- `im::Vector` support for the `List` widget. ([#940] by [@xStrom])
- `TextBox` can receive `EditAction` commands. ([#814] by [@cmyr])
- `Split::min_splitter_area(f64)` to add padding around the splitter bar. ([#738] by [@xStrom])
- Published `druid::text` module. ([#816] by [@cmyr])
- `InternalEvent::MouseLeave` signalling the cursor left the window. ([#821] by [@teddemunnik])
- `InternalEvent::RouteTimer` to route timer events. ([#831] by [@sjoshid])
- `children_changed` now always includes layout and paint request. ([#839] by [@xStrom])
- `request_paint_rect` for partial invalidation. ([#817] by [@jneem])
- Window title can be any `LabelText` (such as a simple `String`). ([#869] by [@cmyr])
- `Label::with_font` and `set_font`. ([#785] by [@thecodewarrior])
- `MouseEvent` now has a `focus` field which is `true` with window focusing left clicks on macOS. ([#842] by [@xStrom])
- `MouseButtons` to `MouseEvent` to track which buttons are being held down during an event. ([#843] by [@xStrom])
- `Env` and `Key` gained methods for inspecting an `Env` at runtime ([#880] by [@Zarenor])
- `WinHandler::scale` method to inform of scale changes. ([#904] by [@xStrom])
- `WidgetExt::debug_widget_id`, for displaying widget ids on hover. ([#876] by [@cmyr])
- `LifeCycle::Size` event to inform widgets that their size changed. ([#953] by [@xStrom])
- `Button::dynamic` constructor. ([#963] by [@totsteps])
- `Spinner` widget to represent loading states. ([#1003] by [@futurepaul])

### Changed

- Renamed `WidgetPod` methods: `paint` to `paint_raw`, `paint_with_offset` to `paint`, `paint_with_offset_always` to `paint_always`. ([#980] by [@totsteps])
- Renamed `Event::MouseMoved` to `MouseMove`. ([#825] by [@teddemunnik])
- Renamed `Split` constructors to `Split::rows` and `columns`. ([#738] by [@xStrom])
- Replaced `NEW_WINDOW`, `SET_MENU` and `SHOW_CONTEXT_MENU` commands with methods on `EventCtx` and `DelegateCtx`. ([#931] by [@finnerale])
- Replaced `Command::one_shot` and `::take_object` with a `SingleUse` payload wrapper type. ([#959] by [@finnerale])
- `Command` and `Selector` have been reworked and are now statically typed, similarly to `Env` and `Key`. ([#993] by [@finnerale])
- `AppDelegate::command` now receives a `Target` instead of a `&Target`. ([#909] by [@xStrom])
- `SHOW_WINDOW` and `CLOSE_WINDOW` commands now only use `Target` to determine the affected window. ([#928] by [@finnerale])
- Global `Application` associated functions are instance methods instead, e.g. `Application::global().quit()` instead of the old `Application::quit()`. ([#763] by [@xStrom])
- `Event::Internal(InternalEvent)` bundles all internal events. ([#833] by [@xStrom])
- Timer events will only be delivered to the widgets that requested them. ([#831] by [@sjoshid])
- `Split::splitter_size` no longer includes padding. ([#738] by [@xStrom])
- `has_focus` no longer returns false positives. ([#819] by [@xStrom])
- `WidgetPod::set_layout_rect` now requires `LayoutCtx`, data and `Env`. ([#841] by [@xStrom])
- `request_timer` uses `Duration` instead of `Instant`. ([#847] by [@finnerale])
- `Event::Wheel` now contains a `MouseEvent` structure. ([#895] by [@teddemunnik])
- The `WindowHandle::get_dpi` method got replaced by `WindowHandle::get_scale`. ([#904] by [@xStrom])
- The `WinHandler::size` method now gets a `Size` in display points. ([#904] by [@xStrom])
- Standardized the type returned by the contexts' `text` methods. ([#996] by [@cmyr])

### Removed

- The optional GTK feature for non-Linux platforms. ([#611] by [@pyroxymat])

### Fixed

- `Event::HotChanged(false)` will be emitted when the cursor leaves the window. ([#821] by [@teddemunnik])
- Keep hot state consistent with mouse position. ([#841] by [@xStrom])
- Start focus cycling from not-registered-for-focus widgets. ([#819] by [@xStrom])
- Supply correct `LifeCycleCtx` to `Event::FocusChanged`. ([#878] by [@cmyr])
- Propagate `Event::FocusChanged` to focus gaining widgets as well. ([#819] by [@xStrom])
- Routing `LifeCycle::FocusChanged` to descendant widgets. ([#925] by [@yrns])
- Focus request handling is now predictable with the last request overriding earlier ones. ([#948] by [@xStrom])
- Open file menu item works again. ([#851] by [@kindlychung])
- Built-in open and save menu items now show the correct label and submit the right commands. ([#930] by [@finnerale])
- Wheel events now properly update hot state. ([#951] by [@xStrom])
- `Painter` now properly repaints on data change in `Container`. ([#991] by [@cmyr])
- Windows: Terminate app when all windows have closed. ([#763] by [@xStrom])
- Windows: Respect the minimum window size. ([#727] by [@teddemunnik])
- Windows: Respect resizability. ([#712] by [@teddemunnik])
- Windows: Capture mouse for drag actions. ([#695] by [@teddemunnik])
- Windows: Removed flashes of white background at the edge of the window when resizing. ([#915] by [@xStrom])
- Windows: Reduced chance of white flash when opening a new window. ([#916] by [@xStrom])
- Windows: Keep receiving mouse events after pressing ALT or F10 when the window has no menu. ([#997] by [@xStrom])
- macOS: `Application::quit` now quits the run loop instead of killing the process. ([#763] by [@xStrom])
- macOS: `Event::HotChanged` is properly generated with multiple windows. ([#907] by [@xStrom])
- macOS: The application menu is now immediately interactable after launch. ([#994] by [@xStrom])
- macOS/GTK: `MouseButton::X1` and `MouseButton::X2` clicks are now recognized. ([#843] by [@xStrom])
- GTK: Use the system locale. ([#798] by [@finnerale])
- GTK: Actually close windows. ([#797] by [@finnerale])
- GTK: Prevent crashing on pop-ups. ([#837] by [@finnerale])
- GTK: Support disabled menu items. ([#897] by [@jneem])
- GTK: Support file filters in open/save dialogs. ([#903] by [@jneem])
- GTK: Support DPI values other than 96. ([#904] by [@xStrom])

### Visual

- Improved `Split` accuracy. ([#738] by [@xStrom])
- Built-in widgets no longer stroke outside their `paint_rect`. ([#861] by [@jneem])
- `Switch` toggles with animation when its data changes externally. ([#898] by [@finnerale])
- Render progress bar correctly. ([#949] by [@scholtzan])
- Scrollbars animate when the scroll container size changes. ([#964] by [@xStrom])

### Docs

- Added example and usage hints to `Env`. ([#796] by [@finnerale])
- Added documentation about the usage of bloom filters. ([#818] by [@xStrom])
- Added book chapters about `Painter` and `Controller`. ([#832] by [@cmyr])
- Added a changelog containing development since the 0.5 release. ([#889] by [@finnerale])
- Added goals section to `README.md`. ([#971] by [@finnerale])
- Added a section about dependencies to `CONTRIBUTING.md`. ([#990] by [@xStrom])
- Updated screenshots in `README.md`. ([#967] by [@xStrom])
- Removed references to cairo on macOS. ([#943] by [@xStrom])

### Examples

- Added `blocking_function`. ([#840] by [@mastfissh])
- Added hot glow option to `multiwin`. ([#845] by [@xStrom])
- Reduce the flashing in `ext_event` and `identity`. ([#782] by [@futurepaul])
- Fixed menu inconsistency across multiple windows in `multiwin`. ([#926] by [@kindlychung])

### Maintenance

- Added rendering tests. ([#784] by [@fishrockz])
- Added docs generation testing for all features. ([#942] by [@xStrom])
- Replaced `#[macro_use]` with normal `use`. ([#808] by [@totsteps])
- Enabled Clippy checks for all targets. ([#850] by [@xStrom])
- Revamped CI testing to optimize coverage and speed. ([#857] by [@xStrom])
- Refactored DPI scaling. ([#904] by [@xStrom])
- Refactored `WidgetPod::event` to improve readability and performance of more complex logic. ([#1001] by [@xStrom])
- Renamed `BaseState` to `WidgetState` ([#969] by [@cmyr])
- Fixed test harness crashing on failure. ([#984] by [@xStrom])
- GTK: Refactored `Application` to use the new structure. ([#892] by [@xStrom])

### Outside News

- There are new projects using druid:
  - [Kondo](https://github.com/tbillington/kondo) Save disk space by cleaning unneeded files from software projects.
  - [jack-mixer](https://github.com/derekdreery/jack-mixer) A jack client that provides mixing, levels and a 3-band eq.
  - [kiro-synth](https://github.com/chris-zen/kiro-synth) An in progress modular sound synthesizer.

## [0.5.0] - 2020-04-01

Last release without a changelog :(

## [0.4.0] - 2019-12-28
## [0.3.2] - 2019-11-05
## [0.3.1] - 2019-11-04
## 0.3.0 - 2019-11-02
## 0.1.1 - 2018-11-02
## 0.1.0 - 2018-11-02

[@futurepaul]: https://github.com/futurepaul
[@finnerale]: https://github.com/finnerale
[@totsteps]: https://github.com/totsteps
[@cmyr]: https://github.com/cmyr
[@xStrom]: https://github.com/xStrom
[@teddemunnik]: https://github.com/teddemunnik
[@crsaracco]: https://github.com/crsaracco
[@pyroxymat]: https://github.com/pyroxymat
[@elrnv]: https://github.com/elrnv
[@tedsta]: https://github.com/tedsta
[@kindlychung]: https://github.com/kindlychung
[@jneem]: https://github.com/jneem
[@fishrockz]: https://github.com/fishrockz
[@thecodewarrior]: https://github.com/thecodewarrior
[@sjoshid]: https://github.com/sjoshid
[@mastfissh]: https://github.com/mastfissh
[@Zarenor]: https://github.com/Zarenor
[@yrns]: https://github.com/yrns
[@jrmuizel]: https://github.com/jrmuizel
[@scholtzan]: https://github.com/scholtzan
[@covercash2]: https://github.com/covercash2

[#599]: https://github.com/xi-editor/druid/pull/599
[#611]: https://github.com/xi-editor/druid/pull/611
[#695]: https://github.com/xi-editor/druid/pull/695
[#712]: https://github.com/xi-editor/druid/pull/712
[#727]: https://github.com/xi-editor/druid/pull/727
[#738]: https://github.com/xi-editor/druid/pull/738
[#759]: https://github.com/xi-editor/druid/pull/759
[#763]: https://github.com/xi-editor/druid/pull/763
[#782]: https://github.com/xi-editor/druid/pull/782
[#784]: https://github.com/xi-editor/druid/pull/784
[#785]: https://github.com/xi-editor/druid/pull/785
[#796]: https://github.com/xi-editor/druid/pull/796
[#797]: https://github.com/xi-editor/druid/pull/797
[#798]: https://github.com/xi-editor/druid/pull/798
[#808]: https://github.com/xi-editor/druid/pull/808
[#814]: https://github.com/xi-editor/druid/pull/814
[#816]: https://github.com/xi-editor/druid/pull/816
[#817]: https://github.com/xi-editor/druid/pull/817
[#818]: https://github.com/xi-editor/druid/pull/818
[#819]: https://github.com/xi-editor/druid/pull/819
[#821]: https://github.com/xi-editor/druid/pull/821
[#825]: https://github.com/xi-editor/druid/pull/825
[#831]: https://github.com/xi-editor/druid/pull/831
[#832]: https://github.com/xi-editor/druid/pull/832
[#833]: https://github.com/xi-editor/druid/pull/833
[#837]: https://github.com/xi-editor/druid/pull/837
[#839]: https://github.com/xi-editor/druid/pull/839
[#840]: https://github.com/xi-editor/druid/pull/840
[#841]: https://github.com/xi-editor/druid/pull/841
[#842]: https://github.com/xi-editor/druid/pull/842
[#843]: https://github.com/xi-editor/druid/pull/843
[#845]: https://github.com/xi-editor/druid/pull/845
[#847]: https://github.com/xi-editor/druid/pull/847
[#850]: https://github.com/xi-editor/druid/pull/850
[#851]: https://github.com/xi-editor/druid/pull/851
[#855]: https://github.com/xi-editor/druid/pull/855
[#857]: https://github.com/xi-editor/druid/pull/857
[#861]: https://github.com/xi-editor/druid/pull/861
[#869]: https://github.com/xi-editor/druid/pull/869
[#876]: https://github.com/xi-editor/druid/pull/876
[#878]: https://github.com/xi-editor/druid/pull/878
[#880]: https://github.com/xi-editor/druid/pull/880
[#889]: https://github.com/xi-editor/druid/pull/889
[#892]: https://github.com/xi-editor/druid/pull/892
[#894]: https://github.com/xi-editor/druid/pull/894
[#895]: https://github.com/xi-editor/druid/pull/895
[#897]: https://github.com/xi-editor/druid/pull/897
[#898]: https://github.com/xi-editor/druid/pull/898
[#900]: https://github.com/xi-editor/druid/pull/900
[#903]: https://github.com/xi-editor/druid/pull/903
[#904]: https://github.com/xi-editor/druid/pull/904
[#905]: https://github.com/xi-editor/druid/pull/905
[#907]: https://github.com/xi-editor/druid/pull/907
[#909]: https://github.com/xi-editor/druid/pull/909
[#915]: https://github.com/xi-editor/druid/pull/915
[#916]: https://github.com/xi-editor/druid/pull/916
[#917]: https://github.com/xi-editor/druid/pull/917
[#920]: https://github.com/xi-editor/druid/pull/920
[#924]: https://github.com/xi-editor/druid/pull/924
[#925]: https://github.com/xi-editor/druid/pull/925
[#926]: https://github.com/xi-editor/druid/pull/926 
[#928]: https://github.com/xi-editor/druid/pull/928
[#930]: https://github.com/xi-editor/druid/pull/930
[#931]: https://github.com/xi-editor/druid/pull/931
[#940]: https://github.com/xi-editor/druid/pull/940
[#942]: https://github.com/xi-editor/druid/pull/942
[#943]: https://github.com/xi-editor/druid/pull/943
[#948]: https://github.com/xi-editor/druid/pull/948
[#949]: https://github.com/xi-editor/druid/pull/949
[#951]: https://github.com/xi-editor/druid/pull/951
[#953]: https://github.com/xi-editor/druid/pull/953
[#954]: https://github.com/xi-editor/druid/pull/954
[#959]: https://github.com/xi-editor/druid/pull/959
[#961]: https://github.com/xi-editor/druid/pull/961
[#963]: https://github.com/xi-editor/druid/pull/963
[#964]: https://github.com/xi-editor/druid/pull/964
[#967]: https://github.com/xi-editor/druid/pull/967
[#969]: https://github.com/xi-editor/druid/pull/969
[#970]: https://github.com/xi-editor/druid/pull/970
[#971]: https://github.com/xi-editor/druid/pull/971
[#972]: https://github.com/xi-editor/druid/pull/972
[#980]: https://github.com/xi-editor/druid/pull/980
[#982]: https://github.com/xi-editor/druid/pull/982
[#984]: https://github.com/xi-editor/druid/pull/984
[#990]: https://github.com/xi-editor/druid/pull/990
[#991]: https://github.com/xi-editor/druid/pull/991
[#993]: https://github.com/xi-editor/druid/pull/993
[#994]: https://github.com/xi-editor/druid/pull/994
[#996]: https://github.com/xi-editor/druid/pull/996
[#997]: https://github.com/xi-editor/druid/pull/997
[#1001]: https://github.com/xi-editor/druid/pull/1001
[#1003]: https://github.com/xi-editor/druid/pull/1003
[#1007]: https://github.com/xi-editor/druid/pull/1007
[#1008]: https://github.com/xi-editor/druid/pull/1008
[#1011]: https://github.com/xi-editor/druid/pull/1011
[#1013]: https://github.com/xi-editor/druid/pull/1013

[Unreleased]: https://github.com/xi-editor/druid/compare/v0.6.0...master
[0.6.0]: https://github.com/xi-editor/druid/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/xi-editor/druid/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/xi-editor/druid/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/xi-editor/druid/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/xi-editor/druid/compare/v0.3.0...v0.3.1
