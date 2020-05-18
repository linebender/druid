# Changelog

## [Unreleased]

### Highlights

#### Basic X11 backend for druid-shell. ([#599])

[@crsaracco] has implemented basic support to run druid on bare-metal X11!

While still incomplete this lays the foundation for running druid on Linux without relying on GTK.

#### Mostly complete Wasm backend for druid-shell. ([#759])

[@elrnv] continued the work of [@tedsta] and implemented a mostly complete Wasm backend and enabled
all druid examples to [run in the browser](https://elrnv.github.io/druid-wasm-examples/).

While some features like the clipboard, menus or file dialogs are not yet available,
all fundamental features are there.

#### Using Core Graphics on macOS. ([#905])

[@cmyr] continued the work of [@jrmuizel] and implemented Core Graphics support for piet in
[piet#176](https://github.com/linebender/piet/pull/176).

Those changes made it into druid via [#905].
This means that druid no longer requires cairo on macOS and uses Core Graphics instead.

### Added

- `TextBox` can receive `EditAction` commands. ([#814] by [@cmyr])
- `Split::min_splitter_area(f64)` to add padding around the splitter bar. ([#738] by [@xStrom])
- Published `druid::text` module. ([#816] by [@cmyr])
- `InternalEvent::MouseLeave` signalling the cursor left the window. ([#821] by [@teddemunnik])
- `children_changed` now always includes layout and paint request. ([#839] by [@xStrom])
- `UpdateCtx::submit_command`. ([#855] by [@cmyr])
- `request_paint_rect` for partial invalidation. ([#817] by [@jneem])
- Window title can be any `LabelText` (such as a simple `String`). ([#869] by [@cmyr])
- `Label::with_font` and `set_font`. ([#785] by [@thecodewarrior])
- `InternalEvent::RouteTimer` to route timer events. ([#831] by [@sjoshid])
- `MouseEvent` now has a `focus` field which is `true` with window focusing left clicks on macOS. ([#842] by [@xStrom])
- `MouseButtons` to `MouseEvent` to track which buttons are being held down during an event. ([#843] by [@xStrom])
- `Env` and `Key` gained methods for inspecting an `Env` at runtime ([#880] by [@Zarenor])
- `UpdateCtx::request_timer` and `UpdateCtx::request_anim_frame`. ([#898] by [@finnerale])
- `LifeCycleCtx::request_timer`. ([#954] by [@xStrom])
- `UpdateCtx::size` and `LifeCycleCtx::size`. ([#917] by [@jneem])
- `WidgetExt::debug_widget_id`, for displaying widget ids on hover. ([#876] by [@cmyr])
- `im` feature, with `Data` support for the [`im` crate](https://docs.rs/im/) collections. ([#924] by [@cmyr])
- `im::Vector` support for the `List` widget. ([#940] by [@xStrom])
- `LifeCycle::Size` event to inform widgets that their size changed. ([#953] by [@xStrom])

### Changed

- Renamed `Split` constructors to `Split::rows` and `columns`. ([#738] by [@xStrom])
- `Split::splitter_size` no longer includes padding. ([#738] by [@xStrom])
- Renamed `Event::MouseMoved` to `MouseMove`. ([#825] by [@teddemunnik])
- `has_focus` no longer returns false positives. ([#819] by [@xStrom])
- `Event::Internal(InternalEvent)` bundles all internal events. ([#833] by [@xStrom])
- `WidgetPod::set_layout_rect` now requires `LayoutCtx`, data and `Env`. ([#841] by [@xStrom])
- `request_timer` uses `Duration` instead of `Instant`. ([#847] by [@finnerale])
- Global `Application` associated functions are instance methods instead, e.g. `Application::global().quit()` instead of the old `Application::quit()`. ([#763] by [@xStrom])
- Timer events will only be delivered to the widgets that requested them. ([#831] by [@sjoshid])
- `Event::Wheel` now contains a `MouseEvent` structure. ([#895] by [@teddemunnik])
- `AppDelegate::command` now receives a `Target` instead of a `&Target`. ([#909] by [@xStrom])
- `SHOW_WINDOW` and `CLOSE_WINDOW` commands now only use `Target` to determine the affected window. ([#928] by [@finnerale])
- Replaced `NEW_WINDOW`, `SET_MENU` and `SHOW_CONTEXT_MENU` commands with methods on `EventCtx` and `DelegateCtx`. ([#931] by [@finnerale])
- Replaced `Command::one_shot` and `::take_object` with a `SingleUse` payload wrapper type. ([#959] by [@finnerale])

### Deprecated

- Nothing

### Removed

- The optional GTK feature for non-Linux platforms. ([#611] by [@pyroxymat])

### Fixed

- GTK: Use the system locale. ([#798] by [@finnerale])
- GTK: Actually close windows. ([#797] by [@finnerale])
- Windows: Respect the minimum window size. ([#727] by [@teddemunnik])
- Windows: Respect resizability. ([#712] by [@teddemunnik])
- `Event::HotChanged(false)` will be emitted when the cursor leaves the window. ([#821] by [@teddemunnik])
- Windows: Capture mouse for drag actions. ([#695] by [@teddemunnik])
- Start focus cycling from non-registered-for-focus widgets. ([#819] by [@xStrom])
- Propagate `Event::FocusChanged` to focus gaining widgets as well. ([#819] by [@xStrom])
- GTK: Prevent crashing on pop-ups. ([#837] by [@finnerale])
- Keep hot state consistent with mouse position. ([#841] by [@xStrom])
- Open file menu item works again. ([#851] by [@kindlychung])
- Supply correct `LifeCycleCtx` to `Event::FocusChanged`. ([#878] by [@cmyr])
- Windows: Termiate app when all windows have closed. ([#763] by [@xStrom])
- macOS: `Application::quit` now quits the run loop instead of killing the process. ([#763] by [@xStrom])
- macOS/GTK/web: `MouseButton::X1` and `MouseButton::X2` clicks are now recognized. ([#843] by [@xStrom])
- GTK: Support disabled menu items. ([#897] by [@jneem])
- X11: Support individual window closing. ([#900] by [@xStrom])
- X11: Support `Application::quit`. ([#900] by [@xStrom])
- GTK: Support file filters in open/save dialogs. ([#903] by [@jneem])
- X11: Support key and mouse button state. ([#920] by [@jneem])
- Routing `LifeCycle::FocusChanged` to descendant widgets. ([#925] by [@yrns])
- Built-in open and save menu items now show the correct label and submit the right commands. ([#930] by [@finnerale])
- Wheel events now properly update hot state. ([#951] by [@xStrom])

### Visual

- Improved `Split` accuracy. ([#738] by [@xStrom])
- Built-in widgets no longer stroke outside their `paint_rect`. ([#861] by [@jneem])
- `Switch` toggles with animation when its data changes externally. ([#898] by [@finnerale])

### Docs

- Reduce the flashing in ext_event and identity examples. ([#782] by [@futurepaul])
- Added example and usage hints to `Env`. ([#796] by [@finnerale])
- Added documentation about the usage of bloom filters. ([#818] by [@xStrom])
- Added Book chapters about `Painter` and `Controller`. ([#832] by [@cmyr])
- Added hot glow option to multiwin example. ([#845] by [@xStrom])
- Added new example for blocking functions. ([#840] by [@mastfissh])
- Added a changelog containing development since the 0.5 release. ([#889] by [@finnerale])
- Removed references to cairo on macOS. ([#943] by [@xStrom])

### Maintenance

- Replaced `#[macro_use]` with normal `use`. ([#808] by [@totsteps])
- Enabled Clippy checks for all targets. ([#850] by [@xStrom])
- Added rendering tests. ([#784] by [@fishrockz])
- Revamped CI testing to optimize coverage and speed. ([#857] by [@xStrom])
- GTK: Refactored `Application` to use the new structure. ([#892] by [@xStrom])
- X11: Refactored `Application` to use the new structure. ([#894] by [@xStrom])
- X11: Refactored `Window` to support some reentrancy and invalidation. ([#894] by [@xStrom])
- Added docs generation testing for all features. ([#942] by [@xStrom])

### Outside News

- There are two new projects using druid:
  - [Kondo](https://github.com/tbillington/kondo) Save disk space by cleaning unneeded files from software projects.
  - [jack-mixer](https://github.com/derekdreery/jack-mixer) A jack client that provides mixing, levels and a 3-band eq.

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
[#905]: https://github.com/xi-editor/druid/pull/905
[#909]: https://github.com/xi-editor/druid/pull/909
[#917]: https://github.com/xi-editor/druid/pull/917
[#920]: https://github.com/xi-editor/druid/pull/920
[#924]: https://github.com/xi-editor/druid/pull/924
[#925]: https://github.com/xi-editor/druid/pull/925
[#928]: https://github.com/xi-editor/druid/pull/928
[#930]: https://github.com/xi-editor/druid/pull/930
[#931]: https://github.com/xi-editor/druid/pull/931
[#940]: https://github.com/xi-editor/druid/pull/940
[#942]: https://github.com/xi-editor/druid/pull/942
[#943]: https://github.com/xi-editor/druid/pull/943
[#951]: https://github.com/xi-editor/druid/pull/951
[#953]: https://github.com/xi-editor/druid/pull/953
[#954]: https://github.com/xi-editor/druid/pull/954
[#959]: https://github.com/xi-editor/druid/pull/959

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

[Unreleased]: https://github.com/xi-editor/druid/compare/v0.5.0...master
[0.5.0]: https://github.com/xi-editor/druid/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/xi-editor/druid/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/xi-editor/druid/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/xi-editor/druid/compare/v0.3.0...v0.3.1
