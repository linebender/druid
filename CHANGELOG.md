# Changelog

The latest published Druid release is [0.8.3](#083---2023-02-28) which was released on 2023-02-28.
You can find its changes [documented below](#083---2023-02-28).

## [Unreleased]

### Highlights

### Added

- Type name is now included in panic error messages in `WidgetPod`. ([#2380] by [@matthewgapp])
- `set_mouse_pass_through` sets whether the mouse passes through the window to whatever is behind. ([#2402] by [@AlexKnauth])
- `is_foreground_window` returns true if the window is the foreground window or this is unknown, and returns false if a different window is known to be the foreground window. ([#2402] by [@AlexKnauth])

### Changed

- Windows: Custom cursor is now encapsulated by `Rc` instead of `Arc`. ([#2409] by [@xStrom])

### Deprecated

### Removed

- `text::format` module which was previously deprecated. ([#2413] by [@xStrom])

### Fixed

- `syn` feature `extra-traits` is now always enabled. ([#2375] by [@AtomicGamer9523])
- Title bar color was opposite of the system theme on Windows. ([#2378] by [@Insprill])

### Visual

### Docs

### Examples

### Maintenance

### Outside News

## [0.8.3] - 2023-02-28

### Added

- Input Region and Always On Top support. ([#2328] by [@jaredoconnell])
- `foreground`, `set_foreground`, and `clear_foreground` methods to `Container` and `WidgetExt::foreground` method for convenience. ([#2346] by [@giannissc])
- `WindowHandle::hide` method to hide a window. ([#2191] by [@newcomb-luke])

### Fixed

- `AddTab` is now properly exported from the `widget` module. ([#2351] by [@cbondurant])

### Docs

- `Widget`, `WidgetExt`, `WidgetId`, `Lens` and `LensExt` docs are visible again. ([#2356] by [@xStrom])
- Deprecated items are now hidden. ([#2356] by [@xStrom])
- Fixed `rustdoc` example scraping configuration. ([#2353] by [@xStrom])
- Added info about git symlinks to `CONTRIBUTING.md`. ([#2349] by [@xStrom])

### Maintenance

- Synchronized `kurbo` and `image` imports with `piet-common`. ([#2352] by [@xStrom])

## [0.8.2] - 2023-01-27

### Docs

- Fixed docs.rs failing to build any docs. ([#2348] by [@xStrom])

## [0.8.1] - 2023-01-27

### Docs

- Fixed `README.md` selection for crates.io. ([#2347] by [@xStrom])

## [0.8.0] - 2023-01-27

### Highlights

#### Text improvements

We now have international text input support (IME) on macOS thanks to [#1619] by [@lord].
The `TextBox` widget was rewritten with IME integration in [#1636] by [@cmyr].
Rich text and complex scripts are now supported on Linux thanks to a major Piet upgrade.

#### Wayland backend

We have a new experimental Wayland backend which can be enabled with the `wayland` feature.
The work was started by [@derekdreery] and then later picked up by [@james-lawrence] in [#2079], [#2114], [#2127] - helped along by [@Maan2003].
Later support was widened in [#2254] by [@PolyMeilex].

### Added

- `RichTextBuilder` for creating `RichText` objects. ([#1520], [#1596] by [@Maan2003])
- `Link` text attribute for links in `RichText`. ([#1627] by [@Maan2003], [#1656] by [@cmyr])
- Strikethrough support in `RichText`. ([#1953] by [@jenra-uwu])
- `StringCursor::new` constructor. ([#2319] by [@benoitryder])
- Ctrl+C, Ctrl+X, Ctrl+V key handling for `TextBox`. ([#1660] by [@cmyr])
- Delete key handling for `TextBox`. ([#1746] by [@bjorn])
- Ctrl+Left, Ctrl+Right, Ctrl+Backspace, Ctrl+Delete, Home, End, PageUp, PageDown key handling for `TextBox`. ([#1786] by [@CryZe])
- Ctrl+A key handling for `TextBox`. ([#1931], [#2031] by [@Maan2003])
- `AppLauncher::localization_resources` for using custom localization resources. ([#1528] by [@edwin0cheng])
- Transparent window support via the `WindowDesc::transparent` method. ([#1583], [#1617] by [@Ciantic] for Windows, [@rjwittams] for macOS, [@JAicewizard] for GTK, and [#1803] by [@psychon] for X11)
- Sub windows - allow opening windows that share state with arbitrary parts of the widget hierarchy. ([#1254] by [@rjwittams])
- `WindowCloseRequested` and `WindowDisconnected` events for when a window is closing. ([#1254] by [@rjwittams])
- `WindowHandle::content_insets`. ([#1532] by [@rjwittams] for macOS; [#1592] by [@HoNile] for Windows; [#1722], [#2117] by [@jneem], [@maurerdietmar] for GTK)
- `window_origin`, `to_window`, and `to_screen` methods to `EventCtx`, `LifeCycleCtx`, `UpdateCtx`, and `PaintCtx`. ([#1532] by [@rjwittams])
- `WindowSizePolicy` for allowing windows to be sized by their content. ([#1532] by [@rjwittams], [#1604] by [@HoNile])
- `WindowDesc::set_level` method. ([#1515] by [@Schaback])
- `WindowDesc::with_config` method to set the `WindowConfig` of the window. ([#1929] by [@Maan2003])
- `Event::WindowScale` to notify widgets of the window's scale changes. ([#2335] by [@xStrom])
- `scale` method to all contexts for widgets to easily access the window's scale. ([#2335] by [@xStrom])
- [`raw-window-handle`](https://docs.rs/raw-window-handle) support via the `raw-win-handle` feature for Windows, macOS, and X11. ([#1586], [#1828] by [@djeedai]; [#1667] by [@i509VCB]; [#2238] by [@Azorlogh])
- `WidgetExt::on_added` method to provide a closure that will be called when the widget is added to the tree. ([#1485] by [@arthmis])
- `set_disabled` method to `EventCtx`, `LifeCycleCtx`, and `UpdateCtx` to disable a widget. ([#1632] by [@xarvic])
- `is_disabled` method to `EventCtx`, `LifeCycleCtx`, `UpdateCtx` and `PaintCtx` to check if a widget is disabled. ([#1632] by [@xarvic])
- `LifeCycle::DisabledChanged` and `InternalLifeCycle::RouteDisabledChanged` events for widget disabled state changes. ([#1632] by [@xarvic])
- `LifeCycle::BuildFocusChain` to update the focus-chain. ([#1632] by [@xarvic])
- `scroll_to_view` and `scroll_area_to_view` methods to `EventCtx`, `LifeCycleCtx` and `UpdateCtx`. ([#1976] by [@xarvic])
- `Notification` can now be sent while handling another `Notification`. ([#1640] by [@cmyr])
- `Notification::route` method to identify the last widget that the notification passed through. ([#1978] by [@xarvic])
- `Notification::warn_if_unused` and `Notification::warn_if_unused_set` for whether there should be a warning when no widget handles the notification. ([#2141] by [@xarvic])
- `DelegateCtx::get_external_handle` method. ([#1526] by [@Maan2003])
- `EventCtx::submit_notification_without_warning` method. ([#2141] by [@xarvic])
- `WidgetPod::has_focus` method to check if the widget or any descendent is focused. ([#1825] by [@ForLoveOfCats])
- `WidgetPod::layout_requested` method to determine whether the widget or any of its children have requested layout. ([#2145] by [@xarvic])
- `Widget::compute_max_intrinsic` method which determines the maximum useful dimension of the widget. ([#2172] by [@sjoshid])
- `LifeCycle::ViewContextChanged` and `InternalLifeCycle::RouteViewContextChanged` events for when a widget's surroundings change. ([#2149] by [@xarvic])
- `ChangeCtx` and `RequestCtx` traits for code generic over contexts. ([#2149] by [@xarvic])
- `should_propagate_to_hidden` method on `Event` and `LifeCycle` enums. ([#1724] by [@xarvic])
- `Value::Other` for using custom types in `Env`. ([#1910] by [@Maan2003])
- `Value::RoundedRectRadii` for specifying the radius of each corner separately. ([#2039] by [@ngugcx])
- `Env::empty` constructor for creating a set of overrides. ([#1837] by [@cmyr])
- `Env::try_set_raw` method to try to set a resolved `Value` for the specified `Key`. ([#1517] by [@cmyr])
- `Key::raw` method for getting the key's raw string value, useful for debugging. ([#1527] by [@cmyr])
- `DebugState` to fetch and process the state of the widget tree for debugging purposes. ([#1890] by [@PoignardAzur])
- `ExtEventSink::add_idle_callback` method as an alternative to commands for mutating app data from a another thread. ([#1955] by [@Maan2003])
- Support for more niche keys like F13 - F24, and various media keys. ([#2044] by [@CryZe])
- `MouseButtons::count` method to get the number of pressed buttons. ([#1500] by [@jjl])
- `Cursor::Pointer` cursor icon. ([#1612] by [@Maan2003])
- `AspectRatioBox` widget that preserves the aspect ratio given to it. ([#1645] by [@arthmis])
- `Maybe` widget for switching between two possible children, for `Data` that is `Option<T>`. ([#1540] by [@derekdreery])
- `DisabledIf` widget wrapper to disable the inner widget based on `Data` and `Env`. ([#1702] by [@xarvic])
- `WidgetWrapper` widget for widgets that wrap a single child to expose that child for access and mutation. ([#1511] by [@rjwittams])
- `ZStack` which is a container that stacks its children on top of each other. ([#2235], [#2291] by [@xarvic])
- `WidgetExt::scroll` method for easily encapsulating a widget in a `Scroll` container. ([#1600] by [@totsteps])
- `content_must_fill`, `set_content_must_fill`, `set_enabled_scrollbars`, `set_vertical_scroll_enabled`, `set_horizontal_scroll_enabled` methods to `Scroll`. ([#1635] by [@cmyr])
- `content_must_fill`, `set_content_must_fill` methods to `ClipBox`. ([#1630] by [@cmyr])
- `ClipBox::unmanaged` constructor for when you are using `ClipBox` in the widget tree directly. ([#2141] by [@xarvic])
- Scroll-to-view support for `ClipBox` and `Tabs`. ([#2141] by [@xarvic])
- `with_tab_index`, `set_tab_index`, and `tab_index` methods to `Tabs` to control the index. ([#2082] by [@rjwittams]
- `state` and `state_mut` methods to `Scope` to get references to the inner state.  ([#2082] by [@rjwittams]
- `Slider::with_step` method for stepping functionality. ([#1875] by [@raymanfx])
- `RangeSlider` and `Annotated` widgets, which are both variations on `Slider`. ([#1979] by [@xarvic])
- `Checkbox::from_label` constructor. ([#2111] by [@maurerdietmar])
- `Container::clear_background` and `Container::clear_border` methods. ([#1558] by [@sjoshid])
- `CrossAxisAlignment::Fill` to the `Flex` widget for filling the entire available cross space. ([#1551] by [@tirix])
- `Data` implementations for more types from `std`. ([#1534] by [@derekdreery])
- `Data` implementations for `Key` and `KeyOrValue`. ([#1571] by [@rjwittams])
- `Data` implementation for `ImageBuf`. ([#1512] by [@arthmis])
- `chrono` feature with `Data` support for [chrono](https://docs.rs/chrono/) types. ([#1743] by [@r-ml])
- Data derive macro attribute `#[data(eq)]`, which is equivalent to `#[data(same_fn = "PartialEq::eq")]`. ([#1884] by [@Maan2003])
- `ListIter` implementation for `OrdMap`. ([#1641] by [@Lejero])
- `ListIter` implementations for `Arc<VecDequeue<T>>` and `(S, Arc<VecDequeue<T>>)`. ([#1639] by [@raymanfx])
- `lens` macro support for accessing nested fields. ([#1764] by [@Maan2003])
- `lens` and `lens_mut` methods to `LensWrap` to get references to the inner lens. ([#1744] by [@SecondFlight])
- `Lens` implementation for `Lens` tuples of length 2-8. ([#1654] by [@Maan2003])
- Type parameters to generated lenses of generic structs, allowing for easier lens usage. ([#1591] by [@rjwittams])
- Derived lens items now get auto-generated documentation. ([#1696] by [@lidin])
- `theme::SCROLLBAR_MIN_SIZE` for specifying the minimum length for any scrollbar. ([#1661] by [@Cupnfish])
- `theme::SELECTED_TEXT_INACTIVE_BACKGROUND_COLOR` for styling text selection in widgets that aren't focused. ([#1659] by [@cmyr])
- Windows: Dark mode detection and titlebar theming. ([#2196], [#2204] by [@dristic])
- Windows: Use custom application icon if the resource is present. ([#2274] by [@tay64])
- Windows: Implemented `Application::get_locale`. ([#1874] by [@dfrg])
- macOS: Implemented save dialog file format selection. ([#1847] by [@terhechte])
- macOS: Implemented `WindowHandle::get_scale`. ([#2297] by [@dfrg])
- Linux: `ApplicationExt::primary_clipboard` method for getting a handle to the primary system clipboard. ([#1843], [#1873] by [@Maan2003])
- GTK: Implemented `WindowHandle::handle_titlebar`. ([#2230] by [@Steve-xmh])
- GTK: Support for selected menu items. ([#2251] by [@longmathemagician])
- GTK: Multiple file selection support in the file dialogs. ([#2081] by [@Psykopear])
- X11: `set_size`, `get_size`, `set_min_size`, `resizable`, `get_position`, `set_position`, `set_level`, and `set_window_state` implementations. ([#1785] by [@Maan2003])
- X11: Support for clipboard. ([#1805], [#1851], [#1866] by [@psychon])
- X11: Implemented `ApplicationExt::primary_clipboard`. ([#1867] by [@psychon])
- X11: Now setting `WM_CLASS` property. ([#1868] by [@psychon])
- X11: Support for DPI scaling. ([#1751] by [@Maan2003])
- X11: Support for changing cursors. ([#1755] by [@Maan2003])
- X11: Support for custom cursors. ([#1801] by [@psychon])
- X11: Support for window focus events. ([#1938] by [@Maan2003]
- X11: Support for file dialogs. ([#2153] by [@jneem])
- X11: Implemented keyboard layout detection. ([#1779] by [@Maan2003])
- X11: Implemented `Screen::get_monitors`. ([#1804] by [@psychon])
- X11: Implemented `Application::get_locale`. ([#1756] by [@Maan2003])
- Web: Implemented `Application::get_locale`. ([#1791] by [@Maan2003])
- OpenBSD: Support for building. ([#1993] by [@klemensn])
- FreeBSD: Support for building. ([#2249] by [@nunotexbsd])

### Changed

- `WindowDesc::new` now takes the root widget directly instead of a function. ([#1559] by [@lassipulkkinen])
- `register_for_focus` must now be called in response to `LifeCycle::BuildFocusChain` instead of `LifeCycle::WidgetAdded`. ([#1632] by [@xarvic])
- Menu support was rewritten with support for `Data`. ([#1625] by [@jneem])
- Window size and positioning code is now in display points. ([#1713] by [@jneem]; [#2297] by [@raphlinus])
- Renamed `druid_shell::platform` module to `druid_shell::backend`. ([#1857] by [@Maan2003])
- `WidgetPod::set_origin` no longer takes `data` and `env` as parameters. ([#2149] by [@xarvic])
- `WidgetPod::event` now propagates handled mouse events to active children. ([#2235] by [@xarvic])
- `WindowLevel` variants `Tooltip`, `DropDown`, and `Modal` now require the parent window's `WindowHandle`. ([#1919] by [@JAicewizard])
- `add_idle_callback` now takes `&mut dyn WinHandler` instead of `&dyn Any`. ([#1787] by [@jneem])
- `AppDelegate::window_added` now receives the new window's `WindowHandle`. ([#2119] by [@zedseven])
- `EventCtx::focus_next`, `EventCtx::focus_prev` and `EventCtx::resign_focus` can now also be called by an ancestor of a focused widget. ([#1651] by [@cmyr])
- `ClipBox::new` constructor was renamed to `ClipBox::managed`. ([#2141] by [@xarvic])
- `ClipBox`, `Flex`, `List` and `Split` only call `layout` on their children when they need it. ([#2145] by [@xarvic])
- Spacers in `Flex` are now implemented by calculating the space in `Flex` instead of creating a widget for it. ([#1584] by [@JAicewizard])
- `Flex` values less than zero will clamp to zero and warn in release mode and panic in debug mode. ([#1691] by [@arthmis])
- `RadioGroup::new` constructor was replaced by `RadioGroup::row`, `RadioGroup::column`, and `RadioGroup::for_axis`. ([#2157] by [@twitchyliquid64])
- `Svg` widget was changed from a simple custom implementation to `resvg` + `tiny-skia` to increase compatibility. ([#2106] by [@james-lawrence], [#2335] by [@xStrom])
- `Slider` widget now warns if `max` < `min` and swaps the values. ([#1882] by [@Maan2003])
- `Padding` is now generic over its child, implements the new `WidgetWrapper` trait. ([#1634] by [@cmyr])
- `Padding::new` now takes `impl Into<KeyOrValue<Insets>>` instead of just `impl Into<Insets>`. ([#1662] by [@cmyr])
- `Container::rounded` and `Container::set_rounded` methods now take `impl Into<KeyOrValue<RoundedRectRadii>>` instead of `impl Into<KeyOrValue<f64>>`. ([#2091] by [@jneem])
- `SizedBox::width` and `SizedBox::height` methods now take `impl Into<KeyOrValue<f64>>` instead of just `f64`. ([#2151] by [@GoldsteinE])
- `TextBox::with_placeholder` and `TextBox::set_placeholder` methods now take `impl Into<LabelText<T>>` instead of `impl Into<String>`. ([#1908] by [@Swatinem])
- `Label::new` now accepts functions that return any type that implements `Into<ArcStr>`. ([#2064] by [@jplatte])
- `PROGRESS_BAR_RADIUS`, `BUTTON_BORDER_RADIUS`, `TEXTBOX_BORDER_RADIUS`, and `SCROLLBAR_RADIUS` in `theme` now use `RoundedRectRadii` instead of `f64`. ([#2039] by [@ngugcx])
- Optimized `ListIter` implementations for `Arc<Vec<T>>`, `(S, Arc<Vec<T>>)`, `Arc<VecDequeue<T>>` and `(S, Arc<VecDequeue<T>>)`. ([#1967] by [@xarvic])
- `Application` can now be restarted. ([#1700] by [@djeedai])
- macOS: `Application::hide`, `Application::hide_others`, `Application::set_menu` methods moved to `ApplicationExt`. ([#1863] by [@Maan2003])
- X11: Query atoms only once instead of per window. ([#1865] by [@psychon])
- Web: Repeated `request_anim_frame` calls are now ignored until the next `request_animation_frame` callback is executed. ([#1790] by [@Maan2003])

### Deprecated

- `AppLauncher::use_simple_logger` in favor of `AppLauncher::log_to_console` and `AppLauncher::start_console_logging`. ([#1578], [#1621] by [@PoignardAzur]; [#2102] by [@ratmice])
- `theme::LABEL_COLOR` in favor of `theme::TEXT_COLOR`. ([#1717] by [@xarvic])
- `theme::SELECTION_COLOR` in favor of `theme::SELECTED_TEXT_BACKGROUND_COLOR`. ([#1659] by [@cmyr])
- `ArcStr`, `FontDescriptor`, `FontFamily`, `FontStyle`, `FontWeight`, `TextAlignment`, `TextLayout` in the `druid` module in favor of `druid::text` module. ([#1689] by [@cmyr])
- `Cursor::OpenHand` because it is not available on Windows. ([#1612] by [@Maan2003])
- `command::HIDE_APPLICATION` and `command::HIDE_OTHERS` on platforms other than macOS. ([#1863] by [@Maan2003])
- `menu::MenuDesc<T>` in favor of `menu::Menu<T>` and its methods `append_entry` and `append_separator` in favor of `entry` and `separator`. ([#1625] by [@jneem])

### Removed

- `WindowHandle::set_level` method. ([#1919] by [@JAicewizard])
- `WidgetPod::set_layout_rect` because it was deprecated and no longer doing what it claimed. ([#2340] by [@xStrom])
- `Default` implementation for `FlexParams`. ([#1885] by [@Maan2003])
- `Default` implementation for `Env` and `theme::init` function. ([#1837] by [@cmyr])

### Fixed

- `TextBox` text clipping. ([#1775] by [@CryZe])
- `TextBox` caret clipping is now more consistent with text clipping. ([#1712] by [@andrewhickman])
- `TextBox` caret not being pixel aligned. ([#1794] by [@CryZe])
- `TextBox` selection alignment offset. ([#1769] by [@CryZe])
- `TextBox` placeholder text alignment. ([#1856] by [@cmyr])
- Single line `TextBox` now resets its scroll position on focus loss. ([#1663] by [@cmyr])
- Double- or triple-clicking on text to select a word/paragraph and then dragging to select even more. ([#1665], [#1666] by [@cmyr])
- Selecting the last word in text. ([#1893] by [@Maan2003])
- `RichText` now invalidates its layout when `Env` changes. ([#1907] by [@Maan2003])
- `Split` no longer causes cursor flicker when the mouse moves fast. ([#1726] by [@djeedai])
- Focus-chain no longer contains hidden widgets. ([#1724] by [@xarvic])
- Scrollbar layout with very small viewports. ([#1715] by [@andrewhickman])
- Scrollbars no longer remain permanently visible when the mouse leaves the window. ([#2343] by [@xStrom])
- Hot state now works properly inside `Scroll`. ([#2149] by [@xarvic])
- `Scroll` now behaves properly inside of `Flex`. ([#1506] by [@tirix])
- `Either` and `Tab` widgets no longer propagate events to hidden children. ([#1860] by [@lisael])
- Panic due to using incorrect flex params in `Tabs`. ([#1740] by [@Maan2003])
- `Painter` now requests paint when the `BackgroundBrush` changes. ([#1881] by [@Maan2003])
- Keep baseline offset in `Align`, `Container`, `Flex`, and `Padding`. ([#2078] by [@maurerdietmar])
- `Env` changes now properly invalidate stale `Container`, `Flex`, and `List`. ([#1894] by [@Maan2003])
- `Data` implementation for `kurbo::RoundedRect`. ([#1618] by [@JAicewizard])
- `Parse` now properly handles floats. ([#2148] by [@superfell])
- `Image` now preserves the aspect ratio of a clipped region. ([#2195] by [@barsae])
- Numerical imprecision in `ClipBox` layout. ([#1776] by [@jneem])
- `serde` feature now properly propagates to the embedded `kurbo` crate. ([#1871] by [@Kethku])
- `Notification` no longer received by the widget that sent it. ([#1640] by [@cmyr])
- `ListIter` implementations for `im::Vector<T>` and `(S, im::Vector<T>)`. ([#1967] by [@xarvic])
- Windows: Alt+Tab now works properly when using Narrator. ([#2026] by [@mwcampbell])
- Windows: Windows without titlebars can now be minimized. ([#2038] by [@ngugcx])
- Windows: No longer panicking on startup when no window size is specified. ([#1575] by [@Perlmint])
- Windows: Now accounting for scale when setting the initial window position. ([#2296] by [@xStrom])
- Windows: Ctrl+Backspace now properly deletes the left word in text. ([#1574] by [@Ciantic])
- Windows: `WindowLevel::Tooltip` no longer steals focus, has a taskbar icon, or an incorrect size. ([#1737] by [@djeedai])
- macOS: Menus are now properly initialized. ([#1846] by [@terhechte])
- macOS: Creating a window with `WindowState::Maximized` now has correct layout. ([#1692] by [@JarrettBillingsley])
- macOS: Mouse leave events are now working properly on macOS 13. ([#2282] by [@liias])
- GTK: Window maximizing works now. ([#2118] by [@Pavel-N])
- GTK: Hot state now properly resets when the mouse leaves the window via an occluded part. ([#2324] by [@xStrom])
- GTK: Avoid undefined behavior when requesting animation frames. ([#1832] by [@JAicewizard])
- GTK: Meta key modifier now works. ([#2293] by [@lzhoucs])
- GTK: Shift+Tab not recognized as a Tab press. ([#1597] by [@cmyr])
- GTK: No longer mangling newline characters in clipboard. ([#1695] by [@ForLoveOfCats])
- GTK: Replaced call to `std::str::from_utf8_unchecked` with `from_utf8`. ([#1820] by [@psychon])
- GTK: `Screen::get_monitors` no longer panics due to GDK not being initialized. ([#1946] by [@JAicewizard])
- X11: `Screen::get_monitors` no longer panics due to `Application` not being initialized. ([#1996] by [@Maan2003])
- Web: Key down events are now handled correctly. ([#1792] by [@Maan2003])

### Visual

- Widgets have a new look and feel when disabled. ([#1717] by [@xarvic])
- `Tabs` widget's close button is now painted with strokes instead of a rendered font. ([#1510] by [@rjwittams])
- `Checkbox` widget's checkmark is now properly centered. ([#2036] by [@agentsim])
- The Druid project itself has a new logo. ([#1550] by [@neurotok], [#1916] by [@PoignardAzur])

### Docs

- Rewrote multiple chapters of the Druid book. ([#2301] by [@PoignardAzur])
- Rewrote the lens chapter of the Druid book. ([#1444] by [@derekdreery])
- Fixed example code in the *Get started with Druid* chapter of the book. ([#1698] by [@ccqpein])
- Added more detailed explanation of `Target::Auto`. ([#1761] by [@arthmis])
- Added code examples to `TextBox` docs. ([#2284] by [@ThomasMcandrew])
- Added a link to the [druid_widget_nursery](https://github.com/linebender/druid-widget-nursery) to `README.md`. ([#1754] by [@xarvic])
- Updated docs of `should_propagate_to_hidden`, `children_changed` and `register_for_focus`. ([#1861] by [@xarvic])
- Updated `Event::AnimFrame` docs with info about when `paint` happens. ([#2323] by [@xStrom])
- Updated `CONTRIBUTING.md` to use `cargo-edit` 0.11. ([#2330] by [@xStrom])
- Fixed docs of derived `Lens`. ([#1523] by [@Maan2003])
- Fixed docs of `RawLabel`. ([#1886] by [@Maan2003])
- Fixed docs describing the `ViewSwitcher` widget functionality. ([#1693] by [@arthmis])
- Fixed all the broken links that originated from Druid crates. ([#2338] by [@xStrom], [#1730] by [@RichardPoole42], [#2158] by [@yrns])
- Removed outdated section in `LifeCycle::WidgetAdded` docs. ([#2320] by [@sprocklem])
- Removed outdated line in `KeyEvent` docs. ([#2247] by [@amtep])
- Implemented standardized Druid naming convention. ([#2337] by [@xStrom])

### Examples

- Added a readme describing all the examples. ([#1423] by [@JAicewizard], [#1992] by [@winksaville])
- Added `markdown_preview` to demonstrate rich text. ([#1513] by [@cmyr])
- Added `transparency` to demonstrate transparent windows. ([#1583] by [@Ciantic])
- Added `slider` to demonstrate `Slider` and `RangeSlider`. ([#1979] by [@xarvic])
- Added `z_stack` to demonstrate `ZStack`. ([#2235] by [@xarvic])
- Added `FillStrat` and `InterpolationMode` usage to the `image` example. ([#1447] by [@JAicewizard])
- Cleaned up `game_of_life`. ([#1443] by [@JAicewizard])
- `open_save` now correctly shows the open dialog when pressing the open button. ([#1914] by [@minimal-state])
- `value_formatting` now correctly handles currency validation errors. ([#1842] by [@cmyr])
- Windows: No longer opening the console when launching the examples. ([#1897] by [@PoignardAzur])

### Maintenance

- Replaced `lazy_static` with `once_cell`. ([#2263] by [@jplatte])
- Updated `piet-common` to 0.6, `kurbo` to 0.9. ([#1677], [#2040] by [@cmyr]; [#1845] by [@JAicewizard]; [#2290] by [@jneem])
- Updated `cairo-rs`, `cairo-sys-rs`, `gdk-sys`, `gtk-rs`, `glib-sys`, `gtk-sys` to 0.16. ([#1845] by [@JAicewizard], [#2290] by [@jneem])
- Updated `x11rb` to 0.10. ([#1519], [#2231] by [@psychon])
- Updated `fluent-bundle` to 0.15 and `fluent-syntax` to 0.11. ([#1772] by [@r-ml])
- Updated `tracing-wasm` to 0.2. ([#1793] by [@Maan2003])
- Updated `tracing-subscriber` to 0.3. ([#2048] by [@jplatte])
- Updated `usvg` to 0.25. ([#1802] by [@r-ml], [#2106] by [@james-lawrence], [#2345] by [@xStrom])
- Updated `time` to 0.3. ([#1969] by [@PoignardAzur])
- Updated `keyboard-types` to 0.6. ([#2044] by [@CryZe])
- Updated `nix` to 0.24. ([#2218] by [@Maan2003])
- Updated `bindgen` to 0.61. ([#2276] by [@jplatte])
- Updated `float-cmp` to 0.9. ([#2329] by [@xStrom])
- Unified window size rounding strategy. ([#2297] by [@xStrom])
- Unified `Selection` and `Movement` into `druid-shell`. ([#1653], [#1655] by [@cmyr])
- Updated Rust edition to 2021 in all Druid crates except `druid-derive`. ([#2327] by [@xStrom])
- Updated source code, tests, and docs to use `Selector::with` instead of `Command::new`. ([#1761] by [@arthmis])
- Switched to trace-based logging, added some tracing and logging. ([#1578], [#1621] by [@PoignardAzur]; [#2203] by [@NickLarsenNZ])
- Converted all calls of `approx_eq!` in tests to `assert_approx_eq!`. ([#2331] by [@cbondurant])
- X11: Added logging to `Application::get_locale`. ([#1876] by [@Maan2003])

## [0.7.0] - 2021-01-01

### Highlights

- Text improvements: `TextLayout` type ([#1182]) and rich text support ([#1245]).
- The `Formatter` trait provides more flexible handling of conversions between
values and their textual representations. ([#1377])

### Added

- `RichText` and `Attribute` types for creating rich text. ([#1255] by [@cmyr])
- `RawLabel` widget for displaying text `Data`. ([#1252] by [@cmyr])
- `TextBox` now supports ctrl and shift hotkeys. ([#1076] by [@vkahl])
- `TextBox` selected text color customization. ([#1093] by [@sysint64])
- `TextBox` vertical movement support. ([#1280] by [@cmyr])
- `TextBox::with_text_color` and `TextBox::set_text_color`. ([#1320] by [@cmyr])
- `TextBox::with_text_alignment` and `TextBox::set_text_alignment`. ([#1371] by [@cmyr])
- `TextLayout` type simplifies drawing text ([#1182] by [@cmyr])
- `TextAlignment` support in `TextLayout` and `Label`. ([#1210] by [@cmyr])
- `LineBreaking` enum allows configuration of label line-breaking. ([#1195] by [@cmyr])
- `Formatter` trait for value formatting. ([#1377] by [@cmyr])
- `Checkbox::set_text` to update the label. ([#1346] by [@finnerale])
- `Button::from_label` to construct a `Button` with a provided `Label`. ([#1226] by [@ForLoveOfCats])
- Widgets can specify a baseline, `Flex` rows can align baselines. ([#1295] by [@cmyr])
- `ClipBox` widget for building scrollable widgets. ([#1248] by [@jneem])
- `Tabs` widget allowing tabbed layouts. ([#1160] by [@rjwittams])
- 'Scope' widget to allow encapsulation of reactive state. ([#1151] by [@rjwittams])
- `ScrollComponent` for ease of adding consistent, customized, scrolling behavior to a widget. ([#1107] by [@ForLoveOfCats])
- `OPEN_PANEL_CANCELLED` and `SAVE_PANEL_CANCELLED` commands. ([#1061] by [@cmyr])
- `BoxConstraints::UNBOUNDED` constant. ([#1126] by [@danieldulaney])
- Close requests from the shell can now be intercepted. ([#1118] by [@jneem], [#1204] by [@psychon], [#1238] by [@tay64])
- `Lens` derive now supports an `ignore` attribute. ([#1133] by [@jneem])
- `Ref` lens that applies `AsRef` and thus allow indexing arrays. ([#1171] by [@finnerale])
- `request_update` method to `EventCtx`. ([#1128] by [@raphlinus])
- `env_changed` and `env_key_changed` methods to `UpdateCtx`. ([#1207] by [@cmyr])
- `request_timer` method to `LayoutCtx`. ([#1278] by [@Majora320])
- `ExtEventSink`s can now be obtained from widget methods. ([#1152] by [@jneem])
- `Command::to` and `Command::target` to set and get a commands target. ([#1185] by [@finnerale])
- `Menu` commands can now choose a custom target. ([#1185] by [@finnerale])
- `Movement::StartOfDocument`, `Movement::EndOfDocument`. ([#1092] by [@sysint64])
- Implementation of `Data` trait for `i128` and `u128` primitive data types. ([#1214] by [@koutoftimer])
- Unit lens for type erased or display only widgets that do not need data. ([#1232] by [@rjwittams])
- `WindowLevel` to control system window Z order, with macOS and GTK implementations. ([#1231] by [@rjwittams])
- `WIDGET_PADDING_HORIZONTAL` and `WIDGET_PADDING_VERTICAL` to `theme` and `Flex::with_default_spacer` / `Flex::add_default_spacer`. ([#1220] by [@cmyr])
- `CONFIGURE_WINDOW` command to allow reconfiguration of an existing window. ([#1235] by [@rjwittams])
- `Event::should_propagate_to_hidden` and `Lifecycle::should_propagate_to_hidden` to determine whether an event should be sent to hidden widgets (e.g. in `Tabs` or `Either`). ([#1351] by [@andrewhickman])
- Custom mouse cursors. ([#1183] by [@jneem])
- `set_cursor` can be called in the `update` method. ([#1361] by [@jneem])
- `WidgetPod::is_initialized` to check if a widget has received `WidgetAdded`. ([#1259] by [@finnerale])
- `Scalable` re-exported under `druid` namespace. ([#1075] by [@ForLoveOfCats])
- Default minimum size to `WindowConfig`. ([#1438] by [@colinfruit])
- Windows: `Screen` module to get information about monitors and the screen. ([#1037] by [@rhzk])
- Windows: Internal functions to handle re-entrancy. ([#1037] by [@rhzk])
- Windows: Ability to create a window with disabled titlebar, maximized or minimized state, and with position. ([#1037] by [@rhzk])
- Windows: Ability to change window state. Toggle titlebar. Change size and position of window. ([#1037], [#1324] by [@rhzk])
- Windows: `handle_titlebar` to allow a custom titlebar to behave like the OS one. ([#1037] by [@rhzk])
- Windows: Dialogs now respect the parameter passed to `force_starting_directory`. ([#1452] by [@MaximilianKoestler])

### Changed

- Keyboard event handling was majorly reworked. ([#1049] by [@raphlinus])
- Now using the new Piet text API. ([#1143] by [@cmyr])
- `Scale::from_scale` to `Scale::new`, and `Scale` methods `scale_x` / `scale_y` to `x` / `y`. ([#1042] by [@xStrom])
- `Container::rounded` takes `KeyOrValue<f64>` instead of `f64`. ([#1054] by [@binomial0])
- `request_anim_frame` no longer invalidates the entire window. ([#1057] by [@jneem])
- `Env::try_get` (and related methods) now return a `Result` instead of an `Option`. ([#1172] by [@cmyr])
- `lens!` macro now uses move semantics for the index. ([#1171] by [@finnerale])
- `Env` now stores `Arc<str>` instead of `String`. ([#1173] by [@cmyr])
- Open and save dialogs now send configurable commands. ([#1463] by [@jneem])
- Replaced uses of `Option<Target>` with the new `Target::Auto`. ([#1185] by [@finnerale])
- Moved the `Target` parameter from `submit_command` to `Command::new` and `Command::to`. ([#1185] by [@finnerale])
- `Movement::RightOfLine` to `Movement::NextLineBreak`, and `Movement::LeftOfLine` to `Movement::PrecedingLineBreak`. ([#1092] by [@sysint64])
- `AnimFrame` was moved from `lifecycle` to `event`. ([#1155] by [@jneem])
- Renamed `ImageData` to `ImageBuf` and moved it to `druid_shell`. ([#1183] by [@jneem])
- Contexts' `text` methods now return `&mut PietText` instead of cloning. ([#1205] by [@cmyr])
- `WindowDesc` decomposed to `PendingWindow` and `WindowConfig` to allow for sub-windows and reconfiguration. ([#1235] by [@rjwittams])
- `LocalizedString` and `LabelText` now use `ArcStr` instead of `String`. ([#1245] by [@cmyr])
- `LensWrap` widget moved into the `widget` module. ([#1251] by [@cmyr])
- `Delegate::command` now returns `Handled` instead of `bool`. ([#1298] by [@jneem])
- `TextBox` now selects all contents when tabbed to on macOS. ([#1283] by [@cmyr])
- All Image formats are now optional, reducing compile time and binary size by default. ([#1340] by [@JAicewizard])
- The `Cursor` API has changed to a stateful one. ([#1433] by [@jneem])
- Part of the `SAVE_FILE` command is now `SAVE_FILE_AS`. ([#1463] by [@jneem])
- `Image` and `ImageData` are now exported by default. ([#1011] by [@covercash2])
- `ViewSwitcher` uses `Data` type constraint instead of `PartialEq`. ([#1112] by [@justinmoon])
- Windows: Reduced flashing when windows are created on high-DPI displays. ([#1272] by [@rhzk])
- Windows: Improved DPI handling. Druid should now redraw correctly when DPI changes. ([#1037] by [@rhzk])
- Windows: A new window is created with the OS default size unless otherwise specified. ([#1037] by [@rhzk])

### Deprecated

- `KeyCode` in favor of `KbKey` and `KeyModifiers` in favor of `Modifiers`. ([#1049] by [@raphlinus])
- `Parse` widget in favor of the `Formatter` trait and `WidgetExt::parse` method in favor of `TextBox::with_formatter`. ([#1377] by [@cmyr])
- `Region::to_rect` method in favor of `Region::bounding_box`. ([#1338] by [@cmyr])
- `theme::init` function in favor of `Env::default` method. ([#1237] by [@totsteps])

### Removed

- `Scale::from_dpi`, `Scale::dpi_x`, and `Scale::dpi_y`. ([#1042] by [@xStrom])
- `Scale::to_px` and `Scale::to_dp`. ([#1075] by [@ForLoveOfCats])

### Fixed

- `ClipBox` now forwards events if any child is active, not just the immediate child. ([#1448] by [@derekdreery])
- `Data` derive now works when type param bounds are defined. ([#1058] by [@chris-zen])
- `update` is now called after all commands. ([#1062] by [@jneem])
- `Align` widget no longer has blurry borders. ([#1091] by [@sysint64])
- `EnvScope` now also updates the `Env` during `Widget::lifecycle`. ([#1100] by [@finnerale])
- `WidgetExt::debug_widget_id` and `debug_paint_layout` now also apply to the widget they are called on. ([#1100] by [@finnerale])
- `ViewSwitcher` now skips the update after switching widgets. ([#1113] by [@finnerale])
- `Key` and `KeyOrValue` derive `Clone`. ([#1119] by [@rjwittams])
- `submit_command` now allowed from the `layout` method. ([#1119] by [@rjwittams])
- Derivation of lenses now allowed for generic types. ([#1120]) by [@rjwittams])
- `Switch` widget's toggle animation is no longer refresh rate dependent. ([#1145] by [@ForLoveOfCats])
- `Image` widget now computes the layout correctly when unbound in one direction. ([#1189] by [@JAicewizard])
- `TextBox` now resets cursor position after being unfocused. ([#1276] by [@sysint64])
- The scroll bar now shows when the contents of a scrollable area change size. ([#1278] by [@Majora320])
- `Either` now uses the correct paint insets. ([#1299] by [@andrewhickman])
- `Either` now correctly passes events to its hidden child. ([#1351] by [@andrewhickman])
- No longer dropping events while showing file dialogs. ([#1302], [#1328] by [@jneem])
- `LifeCycle::WidgetAdded` is now the first event a widget receives. ([#1259] by [@finnerale])
- Various fixes to cross-platform menus. ([#1306] by [@raphlinus])
- Windows: Improved Windows 7 DXGI compatibility. ([#1311] by [@raphlinus])
- Windows: Fixed crash on resize from incompatible resources. ([#1191] by [@raphlinus]])
- Windows: Multi-click now partially fixed. ([#1157] by [@raphlinus])
- Windows: Clipboard is now properly closed. ([#1410] by [@andrewhickman])
- macOS: Fixed timers not firing during modal loop. ([#1028] by [@xStrom])
- GTK: Directory selection now properly ignores file filters. ([#957] by [@xStrom])
- GTK: Fixed crash when receiving an external command while a file dialog is visible. ([#1043] by [@jneem])
- GTK: Fixed `KeyEvent.repeat` being interrupted when releasing another key. ([#1081] by [@raphlinus])
- GTK: Made dependencies optional, facilitating a pure X11 build. ([#1241] by [@finnerale])
- X11: Added support for idle callbacks. ([#1072] by [@jneem])
- X11: Set some more common window properties. ([#1097] by [@psychon])
- X11: Added support for timers. ([#1096] by [@psychon])
- X11: Fixed errors caused by destroyed windows. ([#1103] by [@jneem])

### Visual

- `TextBox` stroke remains inside its `paint_rect`. ([#1007] by [@jneem])

### Docs

- Added a book chapter about resolution independence. ([#913] by [@xStrom])
- Added documentation for the `Image` widget. ([#1018] by [@covercash2])
- Added documentation to `resizable` and `show_titlebar` in `WindowDesc`. ([#1037] by [@rhzk])
- Fixed a link in `druid::command` documentation. ([#1008] by [@covercash2])
- Fixed broken links in `druid::widget::Container` documentation. ([#1357] by [@StarfightLP])

### Examples

- Added `event_viewer` example. ([#1326] by [@cmyr])
- Renamed `ext_event` to `async_event`. ([#1401] by [@JAicewizard])
- Feature requirements now specified in a standard way. ([#1050] by [@xStrom])

### Maintenance

- Standardized web targeting terminology. ([#1013] by [@xStrom])
- Added `debug_panic` macro for when a backtrace is useful but a panic unnecessary. ([#1259] by [@finnerale])
- X11: Ported the X11 backend to [`x11rb`](https://github.com/psychon/x11rb). ([#1025] by [@jneem])

## [0.6.0] - 2020-06-01

### Highlights

#### X11 backend for druid-shell.

[@crsaracco] got us started and implemented basic support to run Druid on bare-metal X11 in [#599].
Additional features got fleshed out in [#894] and [#900] by [@xStrom]
and in [#920], [#961], and [#982] by [@jneem].

While still incomplete this lays the foundation for running Druid on Linux without relying on GTK.

#### Web backend for druid-shell.

[@elrnv] continued the work of [@tedsta] and implemented a mostly complete web backend
via WebAssembly (Wasm) in [#759] and enabled all Druid examples to
[run in the browser](https://elrnv.github.io/druid-wasm-examples/).

While some features like the clipboard, menus or file dialogs are not yet available,
all fundamental features are there.

#### Using Core Graphics on macOS.

[@cmyr] continued the work of [@jrmuizel] and implemented Core Graphics support for Piet in
[piet#176](https://github.com/linebender/piet/pull/176).

Those changes made it into Druid via [#905].
This means that Druid no longer requires Cairo on macOS and uses Core Graphics instead.

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
- `FileDialogOptions` methods `default_name`, `name_label`, `title`, `button_text`, `packages_as_directories`, `force_starting_directory`. ([#960] by [@xStrom])
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
- macOS: Support `FileDialogOptions::default_type`. ([#960] by [@xStrom])
- macOS: Show the save dialog even with `FileDialogOptions` `select_directories` and `multi_selection` set. ([#960] by [@xStrom])
- X11: Support mouse scrolling. ([#961] by [@jneem])
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

- There are new projects using Druid:
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
[@raphlinus]: https://github.com/raphlinus
[@binomial0]: https://github.com/binomial0
[@ForLoveOfCats]: https://github.com/ForLoveOfCats
[@chris-zen]: https://github.com/chris-zen
[@vkahl]: https://github.com/vkahl
[@psychon]: https://github.com/psychon
[@sysint64]: https://github.com/sysint64
[@justinmoon]: https://github.com/justinmoon
[@rjwittams]: https://github.com/rjwittams
[@rhzk]: https://github.com/rhzk
[@koutoftimer]: https://github.com/koutoftimer
[@tay64]: https://github.com/tay64
[@JAicewizard]: https://github.com/JAicewizard
[@andrewhickman]: https://github.com/andrewhickman
[@colinfruit]: https://github.com/colinfruit
[@Maan2003]: https://github.com/Maan2003
[@derekdreery]: https://github.com/derekdreery
[@MaximilianKoestler]: https://github.com/MaximilianKoestler
[@lassipulkkinen]: https://github.com/lassipulkkinen
[@PoignardAzur]: https://github.com/PoignardAzur
[@HoNile]: https://github.com/HoNile
[@SecondFlight]: https://github.com/SecondFlight
[@lord]: https://github.com/lord
[@Lejero]: https://github.com/Lejero
[@lidin]: https://github.com/lidin
[@xarvic]: https://github.com/xarvic
[@arthmis]: https://github.com/arthmis
[@ccqpein]: https://github.com/ccqpein
[@RichardPoole42]: https://github.com/RichardPoole42
[@r-ml]: https://github.com/r-ml
[@djeedai]: https://github.com/djeedai
[@bjorn]: https://github.com/bjorn
[@lisael]: https://github.com/lisael
[@jenra-uwu]: https://github.com/jenra-uwu
[@klemensn]: https://github.com/klemensn
[@agentsim]: https://github.com/agentsim
[@jplatte]: https://github.com/jplatte
[@zedseven]: https://github.com/zedseven
[@Pavel-N]: https://github.com/Pavel-N
[@maurerdietmar]: https://github.com/maurerdietmar
[@superfell]: https://github.com/superfell
[@GoldsteinE]: https://github.com/GoldsteinE
[@twitchyliquid64]: https://github.com/twitchyliquid64
[@dristic]: https://github.com/dristic
[@NickLarsenNZ]: https://github.com/NickLarsenNZ
[@barsae]: https://github.com/barsae
[@amtep]: https://github.com/amtep
[@ThomasMcandrew]: https://github.com/ThomasMcandrew
[@benoitryder]: https://github.com/benoitryder
[@sprocklem]: https://github.com/sprocklem
[@cbondurant]: https://github.com/cbondurant
[@edwin0cheng]: https://github.com/edwin0cheng
[@raymanfx]: https://github.com/raymanfx
[@danieldulaney]: https://github.com/danieldulaney
[@Majora320]: https://github.com/Majora320
[@StarfightLP]: https://github.com/StarfightLP
[@james-lawrence]: https://github.com/james-lawrence
[@Psykopear]: https://github.com/Psykopear
[@jjl]: https://github.com/jjl
[@Schaback]: https://github.com/Schaback
[@tirix]: https://github.com/tirix
[@Ciantic]: https://github.com/Ciantic
[@Azorlogh]: https://github.com/Azorlogh
[@i509VCB]: https://github.com/i509VCB
[@Cupnfish]: https://github.com/Cupnfish
[@CryZe]: https://github.com/CryZe
[@dfrg]: https://github.com/dfrg
[@terhechte]: https://github.com/terhechte
[@minimal-state]: https://github.com/minimal-state
[@Swatinem]: https://github.com/Swatinem
[@mwcampbell]: https://github.com/mwcampbell
[@ngugcx]: https://github.com/ngugcx
[@Kethku]: https://github.com/Kethku
[@neurotok]: https://github.com/neurotok
[@winksaville]: https://github.com/winksaville
[@JarrettBillingsley]: https://github.com/JarrettBillingsley
[@Perlmint]: https://github.com/Perlmint
[@Steve-xmh]: https://github.com/Steve-xmh
[@nunotexbsd]: https://github.com/nunotexbsd
[@PolyMeilex]: https://github.com/PolyMeilex
[@longmathemagician]: https://github.com/longmathemagician
[@liias]: https://github.com/liias
[@lzhoucs]: https://github.com/lzhoucs
[@ratmice]: https://github.com/ratmice
[@jaredoconnell]: https://github.com/jaredoconnell
[@giannissc]: https://github.com/giannissc
[@newcomb-luke]: https://github.com/newcomb-luke
[@AtomicGamer9523]: https://github.com/AtomicGamer9523
[@Insprill]: https://github.com/Insprill
[@matthewgapp]: https://github.com/matthewgapp
[@AlexKnauth]: https://github.com/AlexKnauth

[#599]: https://github.com/linebender/druid/pull/599
[#611]: https://github.com/linebender/druid/pull/611
[#695]: https://github.com/linebender/druid/pull/695
[#712]: https://github.com/linebender/druid/pull/712
[#727]: https://github.com/linebender/druid/pull/727
[#738]: https://github.com/linebender/druid/pull/738
[#759]: https://github.com/linebender/druid/pull/759
[#763]: https://github.com/linebender/druid/pull/763
[#782]: https://github.com/linebender/druid/pull/782
[#784]: https://github.com/linebender/druid/pull/784
[#785]: https://github.com/linebender/druid/pull/785
[#796]: https://github.com/linebender/druid/pull/796
[#797]: https://github.com/linebender/druid/pull/797
[#798]: https://github.com/linebender/druid/pull/798
[#808]: https://github.com/linebender/druid/pull/808
[#814]: https://github.com/linebender/druid/pull/814
[#816]: https://github.com/linebender/druid/pull/816
[#817]: https://github.com/linebender/druid/pull/817
[#818]: https://github.com/linebender/druid/pull/818
[#819]: https://github.com/linebender/druid/pull/819
[#821]: https://github.com/linebender/druid/pull/821
[#825]: https://github.com/linebender/druid/pull/825
[#831]: https://github.com/linebender/druid/pull/831
[#832]: https://github.com/linebender/druid/pull/832
[#833]: https://github.com/linebender/druid/pull/833
[#837]: https://github.com/linebender/druid/pull/837
[#839]: https://github.com/linebender/druid/pull/839
[#840]: https://github.com/linebender/druid/pull/840
[#841]: https://github.com/linebender/druid/pull/841
[#842]: https://github.com/linebender/druid/pull/842
[#843]: https://github.com/linebender/druid/pull/843
[#845]: https://github.com/linebender/druid/pull/845
[#847]: https://github.com/linebender/druid/pull/847
[#850]: https://github.com/linebender/druid/pull/850
[#851]: https://github.com/linebender/druid/pull/851
[#855]: https://github.com/linebender/druid/pull/855
[#857]: https://github.com/linebender/druid/pull/857
[#861]: https://github.com/linebender/druid/pull/861
[#869]: https://github.com/linebender/druid/pull/869
[#876]: https://github.com/linebender/druid/pull/876
[#878]: https://github.com/linebender/druid/pull/878
[#880]: https://github.com/linebender/druid/pull/880
[#889]: https://github.com/linebender/druid/pull/889
[#892]: https://github.com/linebender/druid/pull/892
[#894]: https://github.com/linebender/druid/pull/894
[#895]: https://github.com/linebender/druid/pull/895
[#897]: https://github.com/linebender/druid/pull/897
[#898]: https://github.com/linebender/druid/pull/898
[#900]: https://github.com/linebender/druid/pull/900
[#903]: https://github.com/linebender/druid/pull/903
[#904]: https://github.com/linebender/druid/pull/904
[#905]: https://github.com/linebender/druid/pull/905
[#907]: https://github.com/linebender/druid/pull/907
[#909]: https://github.com/linebender/druid/pull/909
[#913]: https://github.com/linebender/druid/pull/913
[#915]: https://github.com/linebender/druid/pull/915
[#916]: https://github.com/linebender/druid/pull/916
[#917]: https://github.com/linebender/druid/pull/917
[#920]: https://github.com/linebender/druid/pull/920
[#924]: https://github.com/linebender/druid/pull/924
[#925]: https://github.com/linebender/druid/pull/925
[#926]: https://github.com/linebender/druid/pull/926
[#928]: https://github.com/linebender/druid/pull/928
[#930]: https://github.com/linebender/druid/pull/930
[#931]: https://github.com/linebender/druid/pull/931
[#940]: https://github.com/linebender/druid/pull/940
[#942]: https://github.com/linebender/druid/pull/942
[#943]: https://github.com/linebender/druid/pull/943
[#948]: https://github.com/linebender/druid/pull/948
[#949]: https://github.com/linebender/druid/pull/949
[#951]: https://github.com/linebender/druid/pull/951
[#953]: https://github.com/linebender/druid/pull/953
[#954]: https://github.com/linebender/druid/pull/954
[#957]: https://github.com/linebender/druid/pull/957
[#959]: https://github.com/linebender/druid/pull/959
[#960]: https://github.com/linebender/druid/pull/960
[#961]: https://github.com/linebender/druid/pull/961
[#963]: https://github.com/linebender/druid/pull/963
[#964]: https://github.com/linebender/druid/pull/964
[#967]: https://github.com/linebender/druid/pull/967
[#969]: https://github.com/linebender/druid/pull/969
[#970]: https://github.com/linebender/druid/pull/970
[#971]: https://github.com/linebender/druid/pull/971
[#972]: https://github.com/linebender/druid/pull/972
[#980]: https://github.com/linebender/druid/pull/980
[#982]: https://github.com/linebender/druid/pull/982
[#984]: https://github.com/linebender/druid/pull/984
[#990]: https://github.com/linebender/druid/pull/990
[#991]: https://github.com/linebender/druid/pull/991
[#993]: https://github.com/linebender/druid/pull/993
[#994]: https://github.com/linebender/druid/pull/994
[#996]: https://github.com/linebender/druid/pull/996
[#997]: https://github.com/linebender/druid/pull/997
[#1001]: https://github.com/linebender/druid/pull/1001
[#1003]: https://github.com/linebender/druid/pull/1003
[#1007]: https://github.com/linebender/druid/pull/1007
[#1008]: https://github.com/linebender/druid/pull/1008
[#1011]: https://github.com/linebender/druid/pull/1011
[#1013]: https://github.com/linebender/druid/pull/1013
[#1018]: https://github.com/linebender/druid/pull/1018
[#1025]: https://github.com/linebender/druid/pull/1025
[#1028]: https://github.com/linebender/druid/pull/1028
[#1037]: https://github.com/linebender/druid/pull/1037
[#1042]: https://github.com/linebender/druid/pull/1042
[#1043]: https://github.com/linebender/druid/pull/1043
[#1049]: https://github.com/linebender/druid/pull/1049
[#1050]: https://github.com/linebender/druid/pull/1050
[#1054]: https://github.com/linebender/druid/pull/1054
[#1057]: https://github.com/linebender/druid/pull/1057
[#1058]: https://github.com/linebender/druid/pull/1058
[#1061]: https://github.com/linebender/druid/pull/1061
[#1062]: https://github.com/linebender/druid/pull/1062
[#1072]: https://github.com/linebender/druid/pull/1072
[#1075]: https://github.com/linebender/druid/pull/1075
[#1076]: https://github.com/linebender/druid/pull/1076
[#1081]: https://github.com/linebender/druid/pull/1081
[#1091]: https://github.com/linebender/druid/pull/1091
[#1093]: https://github.com/linebender/druid/pull/1093
[#1096]: https://github.com/linebender/druid/pull/1096
[#1097]: https://github.com/linebender/druid/pull/1097
[#1100]: https://github.com/linebender/druid/pull/1100
[#1103]: https://github.com/linebender/druid/pull/1103
[#1107]: https://github.com/linebender/druid/pull/1107
[#1113]: https://github.com/linebender/druid/pull/1113
[#1118]: https://github.com/linebender/druid/pull/1118
[#1119]: https://github.com/linebender/druid/pull/1119
[#1120]: https://github.com/linebender/druid/pull/1120
[#1126]: https://github.com/linebender/druid/pull/1126
[#1128]: https://github.com/linebender/druid/pull/1128
[#1133]: https://github.com/linebender/druid/pull/1133
[#1143]: https://github.com/linebender/druid/pull/1143
[#1145]: https://github.com/linebender/druid/pull/1145
[#1151]: https://github.com/linebender/druid/pull/1151
[#1152]: https://github.com/linebender/druid/pull/1152
[#1155]: https://github.com/linebender/druid/pull/1155
[#1157]: https://github.com/linebender/druid/pull/1157
[#1160]: https://github.com/linebender/druid/pull/1160
[#1171]: https://github.com/linebender/druid/pull/1171
[#1172]: https://github.com/linebender/druid/pull/1172
[#1173]: https://github.com/linebender/druid/pull/1173
[#1182]: https://github.com/linebender/druid/pull/1182
[#1183]: https://github.com/linebender/druid/pull/1183
[#1185]: https://github.com/linebender/druid/pull/1185
[#1191]: https://github.com/linebender/druid/pull/1191
[#1092]: https://github.com/linebender/druid/pull/1092
[#1189]: https://github.com/linebender/druid/pull/1189
[#1195]: https://github.com/linebender/druid/pull/1195
[#1204]: https://github.com/linebender/druid/pull/1204
[#1205]: https://github.com/linebender/druid/pull/1205
[#1207]: https://github.com/linebender/druid/pull/1207
[#1210]: https://github.com/linebender/druid/pull/1210
[#1214]: https://github.com/linebender/druid/pull/1214
[#1220]: https://github.com/linebender/druid/pull/1220
[#1226]: https://github.com/linebender/druid/pull/1226
[#1231]: https://github.com/linebender/druid/pull/1231
[#1232]: https://github.com/linebender/druid/pull/1232
[#1235]: https://github.com/linebender/druid/pull/1235
[#1237]: https://github.com/linebender/druid/pull/1237
[#1238]: https://github.com/linebender/druid/pull/1238
[#1241]: https://github.com/linebender/druid/pull/1241
[#1245]: https://github.com/linebender/druid/pull/1245
[#1248]: https://github.com/linebender/druid/pull/1248
[#1251]: https://github.com/linebender/druid/pull/1251
[#1252]: https://github.com/linebender/druid/pull/1252
[#1254]: https://github.com/linebender/druid/pull/1254
[#1255]: https://github.com/linebender/druid/pull/1255
[#1272]: https://github.com/linebender/druid/pull/1272
[#1276]: https://github.com/linebender/druid/pull/1276
[#1278]: https://github.com/linebender/druid/pull/1278
[#1280]: https://github.com/linebender/druid/pull/1280
[#1283]: https://github.com/linebender/druid/pull/1283
[#1295]: https://github.com/linebender/druid/pull/1295
[#1298]: https://github.com/linebender/druid/pull/1298
[#1299]: https://github.com/linebender/druid/pull/1299
[#1302]: https://github.com/linebender/druid/pull/1302
[#1306]: https://github.com/linebender/druid/pull/1306
[#1311]: https://github.com/linebender/druid/pull/1311
[#1320]: https://github.com/linebender/druid/pull/1320
[#1324]: https://github.com/linebender/druid/pull/1324
[#1326]: https://github.com/linebender/druid/pull/1326
[#1328]: https://github.com/linebender/druid/pull/1328
[#1338]: https://github.com/linebender/druid/pull/1338
[#1340]: https://github.com/linebender/druid/pull/1340
[#1346]: https://github.com/linebender/druid/pull/1346
[#1351]: https://github.com/linebender/druid/pull/1351
[#1357]: https://github.com/linebender/druid/pull/1357
[#1259]: https://github.com/linebender/druid/pull/1259
[#1361]: https://github.com/linebender/druid/pull/1361
[#1371]: https://github.com/linebender/druid/pull/1371
[#1377]: https://github.com/linebender/druid/pull/1377
[#1401]: https://github.com/linebender/druid/pull/1401
[#1410]: https://github.com/linebender/druid/pull/1410
[#1423]: https://github.com/linebender/druid/pull/1423
[#1433]: https://github.com/linebender/druid/pull/1433
[#1438]: https://github.com/linebender/druid/pull/1438
[#1443]: https://github.com/linebender/druid/pull/1443
[#1444]: https://github.com/linebender/druid/pull/1444
[#1447]: https://github.com/linebender/druid/pull/1447
[#1448]: https://github.com/linebender/druid/pull/1448
[#1452]: https://github.com/linebender/druid/pull/1452
[#1463]: https://github.com/linebender/druid/pull/1463
[#1485]: https://github.com/linebender/druid/pull/1485
[#1500]: https://github.com/linebender/druid/pull/1500
[#1506]: https://github.com/linebender/druid/pull/1506
[#1510]: https://github.com/linebender/druid/pull/1510
[#1511]: https://github.com/linebender/druid/pull/1511
[#1512]: https://github.com/linebender/druid/pull/1512
[#1513]: https://github.com/linebender/druid/pull/1513
[#1515]: https://github.com/linebender/druid/pull/1515
[#1517]: https://github.com/linebender/druid/pull/1517
[#1519]: https://github.com/linebender/druid/pull/1519
[#1520]: https://github.com/linebender/druid/pull/1520
[#1523]: https://github.com/linebender/druid/pull/1523
[#1526]: https://github.com/linebender/druid/pull/1526
[#1527]: https://github.com/linebender/druid/pull/1527
[#1528]: https://github.com/linebender/druid/pull/1528
[#1532]: https://github.com/linebender/druid/pull/1532
[#1534]: https://github.com/linebender/druid/pull/1534
[#1540]: https://github.com/linebender/druid/pull/1540
[#1550]: https://github.com/linebender/druid/pull/1550
[#1551]: https://github.com/linebender/druid/pull/1551
[#1558]: https://github.com/linebender/druid/pull/1558
[#1559]: https://github.com/linebender/druid/pull/1559
[#1571]: https://github.com/linebender/druid/pull/1571
[#1574]: https://github.com/linebender/druid/pull/1574
[#1575]: https://github.com/linebender/druid/pull/1575
[#1578]: https://github.com/linebender/druid/pull/1578
[#1583]: https://github.com/linebender/druid/pull/1583
[#1584]: https://github.com/linebender/druid/pull/1584
[#1591]: https://github.com/linebender/druid/pull/1591
[#1592]: https://github.com/linebender/druid/pull/1592
[#1596]: https://github.com/linebender/druid/pull/1596
[#1597]: https://github.com/linebender/druid/pull/1597
[#1600]: https://github.com/linebender/druid/pull/1600
[#1604]: https://github.com/linebender/druid/pull/1604
[#1612]: https://github.com/linebender/druid/pull/1612
[#1617]: https://github.com/linebender/druid/pull/1617
[#1618]: https://github.com/linebender/druid/pull/1618
[#1619]: https://github.com/linebender/druid/pull/1619
[#1621]: https://github.com/linebender/druid/pull/1621
[#1625]: https://github.com/linebender/druid/pull/1625
[#1627]: https://github.com/linebender/druid/pull/1627
[#1630]: https://github.com/linebender/druid/pull/1630
[#1632]: https://github.com/linebender/druid/pull/1632
[#1634]: https://github.com/linebender/druid/pull/1634
[#1635]: https://github.com/linebender/druid/pull/1635
[#1636]: https://github.com/linebender/druid/pull/1636
[#1639]: https://github.com/linebender/druid/pull/1639
[#1640]: https://github.com/linebender/druid/pull/1640
[#1641]: https://github.com/linebender/druid/pull/1641
[#1645]: https://github.com/linebender/druid/pull/1645
[#1651]: https://github.com/linebender/druid/pull/1651
[#1653]: https://github.com/linebender/druid/pull/1653
[#1654]: https://github.com/linebender/druid/pull/1654
[#1655]: https://github.com/linebender/druid/pull/1655
[#1656]: https://github.com/linebender/druid/pull/1656
[#1659]: https://github.com/linebender/druid/pull/1659
[#1660]: https://github.com/linebender/druid/pull/1660
[#1661]: https://github.com/linebender/druid/pull/1661
[#1662]: https://github.com/linebender/druid/pull/1662
[#1663]: https://github.com/linebender/druid/pull/1663
[#1665]: https://github.com/linebender/druid/pull/1665
[#1666]: https://github.com/linebender/druid/pull/1666
[#1667]: https://github.com/linebender/druid/pull/1667
[#1677]: https://github.com/linebender/druid/pull/1677
[#1689]: https://github.com/linebender/druid/pull/1689
[#1691]: https://github.com/linebender/druid/pull/1691
[#1692]: https://github.com/linebender/druid/pull/1692
[#1693]: https://github.com/linebender/druid/pull/1693
[#1695]: https://github.com/linebender/druid/pull/1695
[#1696]: https://github.com/linebender/druid/pull/1696
[#1698]: https://github.com/linebender/druid/pull/1698
[#1700]: https://github.com/linebender/druid/pull/1700
[#1702]: https://github.com/linebender/druid/pull/1702
[#1712]: https://github.com/linebender/druid/pull/1712
[#1713]: https://github.com/linebender/druid/pull/1713
[#1715]: https://github.com/linebender/druid/pull/1715
[#1717]: https://github.com/linebender/druid/pull/1717
[#1722]: https://github.com/linebender/druid/pull/1722
[#1724]: https://github.com/linebender/druid/pull/1724
[#1726]: https://github.com/linebender/druid/pull/1726
[#1730]: https://github.com/linebender/druid/pull/1730
[#1737]: https://github.com/linebender/druid/pull/1737
[#1740]: https://github.com/linebender/druid/pull/1740
[#1743]: https://github.com/linebender/druid/pull/1743
[#1744]: https://github.com/linebender/druid/pull/1744
[#1746]: https://github.com/linebender/druid/pull/1746
[#1751]: https://github.com/linebender/druid/pull/1751
[#1754]: https://github.com/linebender/druid/pull/1754
[#1755]: https://github.com/linebender/druid/pull/1755
[#1756]: https://github.com/linebender/druid/pull/1756
[#1761]: https://github.com/linebender/druid/pull/1761
[#1764]: https://github.com/linebender/druid/pull/1764
[#1769]: https://github.com/linebender/druid/pull/1769
[#1772]: https://github.com/linebender/druid/pull/1772
[#1775]: https://github.com/linebender/druid/pull/1775
[#1776]: https://github.com/linebender/druid/pull/1776
[#1779]: https://github.com/linebender/druid/pull/1779
[#1785]: https://github.com/linebender/druid/pull/1785
[#1786]: https://github.com/linebender/druid/pull/1786
[#1787]: https://github.com/linebender/druid/pull/1787
[#1790]: https://github.com/linebender/druid/pull/1790
[#1791]: https://github.com/linebender/druid/pull/1791
[#1792]: https://github.com/linebender/druid/pull/1792
[#1793]: https://github.com/linebender/druid/pull/1793
[#1794]: https://github.com/linebender/druid/pull/1794
[#1801]: https://github.com/linebender/druid/pull/1801
[#1802]: https://github.com/linebender/druid/pull/1802
[#1803]: https://github.com/linebender/druid/pull/1803
[#1804]: https://github.com/linebender/druid/pull/1804
[#1805]: https://github.com/linebender/druid/pull/1805
[#1820]: https://github.com/linebender/druid/pull/1820
[#1825]: https://github.com/linebender/druid/pull/1825
[#1828]: https://github.com/linebender/druid/pull/1828
[#1832]: https://github.com/linebender/druid/pull/1832
[#1837]: https://github.com/linebender/druid/pull/1837
[#1842]: https://github.com/linebender/druid/pull/1842
[#1843]: https://github.com/linebender/druid/pull/1843
[#1845]: https://github.com/linebender/druid/pull/1845
[#1846]: https://github.com/linebender/druid/pull/1846
[#1847]: https://github.com/linebender/druid/pull/1847
[#1851]: https://github.com/linebender/druid/pull/1851
[#1856]: https://github.com/linebender/druid/pull/1856
[#1857]: https://github.com/linebender/druid/pull/1857
[#1860]: https://github.com/linebender/druid/pull/1860
[#1861]: https://github.com/linebender/druid/pull/1861
[#1863]: https://github.com/linebender/druid/pull/1863
[#1865]: https://github.com/linebender/druid/pull/1865
[#1866]: https://github.com/linebender/druid/pull/1866
[#1867]: https://github.com/linebender/druid/pull/1867
[#1868]: https://github.com/linebender/druid/pull/1868
[#1871]: https://github.com/linebender/druid/pull/1871
[#1873]: https://github.com/linebender/druid/pull/1873
[#1874]: https://github.com/linebender/druid/pull/1874
[#1875]: https://github.com/linebender/druid/pull/1875
[#1876]: https://github.com/linebender/druid/pull/1876
[#1881]: https://github.com/linebender/druid/pull/1881
[#1882]: https://github.com/linebender/druid/pull/1882
[#1884]: https://github.com/linebender/druid/pull/1884
[#1885]: https://github.com/linebender/druid/pull/1885
[#1886]: https://github.com/linebender/druid/pull/1886
[#1890]: https://github.com/linebender/druid/pull/1890
[#1893]: https://github.com/linebender/druid/pull/1893
[#1894]: https://github.com/linebender/druid/pull/1894
[#1897]: https://github.com/linebender/druid/pull/1897
[#1907]: https://github.com/linebender/druid/pull/1907
[#1908]: https://github.com/linebender/druid/pull/1908
[#1910]: https://github.com/linebender/druid/pull/1910
[#1914]: https://github.com/linebender/druid/pull/1914
[#1916]: https://github.com/linebender/druid/pull/1916
[#1919]: https://github.com/linebender/druid/pull/1919
[#1929]: https://github.com/linebender/druid/pull/1929
[#1931]: https://github.com/linebender/druid/pull/1931
[#1938]: https://github.com/linebender/druid/pull/1938
[#1946]: https://github.com/linebender/druid/pull/1946
[#1953]: https://github.com/linebender/druid/pull/1953
[#1955]: https://github.com/linebender/druid/pull/1955
[#1967]: https://github.com/linebender/druid/pull/1967
[#1969]: https://github.com/linebender/druid/pull/1969
[#1976]: https://github.com/linebender/druid/pull/1976
[#1978]: https://github.com/linebender/druid/pull/1978
[#1979]: https://github.com/linebender/druid/pull/1979
[#1992]: https://github.com/linebender/druid/pull/1992
[#1993]: https://github.com/linebender/druid/pull/1993
[#1996]: https://github.com/linebender/druid/pull/1996
[#2026]: https://github.com/linebender/druid/pull/2026
[#2031]: https://github.com/linebender/druid/pull/2031
[#2036]: https://github.com/linebender/druid/pull/2036
[#2038]: https://github.com/linebender/druid/pull/2038
[#2039]: https://github.com/linebender/druid/pull/2039
[#2040]: https://github.com/linebender/druid/pull/2040
[#2044]: https://github.com/linebender/druid/pull/2044
[#2048]: https://github.com/linebender/druid/pull/2048
[#2064]: https://github.com/linebender/druid/pull/2064
[#2078]: https://github.com/linebender/druid/pull/2078
[#2079]: https://github.com/linebender/druid/pull/2079
[#2082]: https://github.com/linebender/druid/pull/2082
[#2091]: https://github.com/linebender/druid/pull/2091
[#2102]: https://github.com/linebender/druid/pull/2102
[#2106]: https://github.com/linebender/druid/pull/2106
[#2111]: https://github.com/linebender/druid/pull/2111
[#2114]: https://github.com/linebender/druid/pull/2114
[#2117]: https://github.com/linebender/druid/pull/2117
[#2118]: https://github.com/linebender/druid/pull/2118
[#2119]: https://github.com/linebender/druid/pull/2119
[#2127]: https://github.com/linebender/druid/pull/2127
[#2141]: https://github.com/linebender/druid/pull/2141
[#2145]: https://github.com/linebender/druid/pull/2145
[#2148]: https://github.com/linebender/druid/pull/2148
[#2149]: https://github.com/linebender/druid/pull/2149
[#2151]: https://github.com/linebender/druid/pull/2151
[#2153]: https://github.com/linebender/druid/pull/2153
[#2157]: https://github.com/linebender/druid/pull/2157
[#2158]: https://github.com/linebender/druid/pull/2158
[#2172]: https://github.com/linebender/druid/pull/2172
[#2191]: https://github.com/linebender/druid/pull/2191
[#2195]: https://github.com/linebender/druid/pull/2195
[#2196]: https://github.com/linebender/druid/pull/2196
[#2203]: https://github.com/linebender/druid/pull/2203
[#2204]: https://github.com/linebender/druid/pull/2204
[#2218]: https://github.com/linebender/druid/pull/2218
[#2230]: https://github.com/linebender/druid/pull/2230
[#2231]: https://github.com/linebender/druid/pull/2231
[#2235]: https://github.com/linebender/druid/pull/2235
[#2238]: https://github.com/linebender/druid/pull/2238
[#2247]: https://github.com/linebender/druid/pull/2247
[#2249]: https://github.com/linebender/druid/pull/2249
[#2251]: https://github.com/linebender/druid/pull/2251
[#2254]: https://github.com/linebender/druid/pull/2254
[#2263]: https://github.com/linebender/druid/pull/2263
[#2274]: https://github.com/linebender/druid/pull/2274
[#2276]: https://github.com/linebender/druid/pull/2276
[#2282]: https://github.com/linebender/druid/pull/2282
[#2284]: https://github.com/linebender/druid/pull/2284
[#2290]: https://github.com/linebender/druid/pull/2290
[#2291]: https://github.com/linebender/druid/pull/2291
[#2293]: https://github.com/linebender/druid/pull/2293
[#2296]: https://github.com/linebender/druid/pull/2296
[#2297]: https://github.com/linebender/druid/pull/2297
[#2301]: https://github.com/linebender/druid/pull/2301
[#2319]: https://github.com/linebender/druid/pull/2319
[#2320]: https://github.com/linebender/druid/pull/2320
[#2323]: https://github.com/linebender/druid/pull/2323
[#2324]: https://github.com/linebender/druid/pull/2324
[#2327]: https://github.com/linebender/druid/pull/2327
[#2328]: https://github.com/linebender/druid/pull/2328
[#2329]: https://github.com/linebender/druid/pull/2329
[#2330]: https://github.com/linebender/druid/pull/2330
[#2331]: https://github.com/linebender/druid/pull/2331
[#2335]: https://github.com/linebender/druid/pull/2335
[#2337]: https://github.com/linebender/druid/pull/2337
[#2338]: https://github.com/linebender/druid/pull/2338
[#2340]: https://github.com/linebender/druid/pull/2340
[#2343]: https://github.com/linebender/druid/pull/2343
[#2345]: https://github.com/linebender/druid/pull/2345
[#2346]: https://github.com/linebender/druid/pull/2346
[#2347]: https://github.com/linebender/druid/pull/2347
[#2348]: https://github.com/linebender/druid/pull/2348
[#2349]: https://github.com/linebender/druid/pull/2349
[#2351]: https://github.com/linebender/druid/pull/2351
[#2352]: https://github.com/linebender/druid/pull/2352
[#2353]: https://github.com/linebender/druid/pull/2353
[#2356]: https://github.com/linebender/druid/pull/2356
[#2375]: https://github.com/linebender/druid/pull/2375
[#2378]: https://github.com/linebender/druid/pull/2378
[#2380]: https://github.com/linebender/druid/pull/2380
[#2402]: https://github.com/linebender/druid/pull/2402
[#2409]: https://github.com/linebender/druid/pull/2409
[#2413]: https://github.com/linebender/druid/pull/2413

[Unreleased]: https://github.com/linebender/druid/compare/v0.8.3...master
[0.8.3]: https://github.com/linebender/druid/compare/v0.8.2...v0.8.3
[0.8.2]: https://github.com/linebender/druid/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/linebender/druid/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/linebender/druid/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/linebender/druid/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/linebender/druid/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/linebender/druid/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/linebender/druid/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/linebender/druid/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/linebender/druid/compare/v0.3.0...v0.3.1
