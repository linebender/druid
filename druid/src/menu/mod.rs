// Copyright 2019 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! ## Window, application, and context menus
//!
//! Menus in Druid follow a data-driven design similar to that of the main widget tree. The main
//! types are [`Menu`] (representing a tree of menus and submenus) and [`MenuItem`] (representing a
//! single "leaf" element).
//!
//! ## Menu actions
//!
//! Menu items can be associated with callbacks, which are triggered when a user selects that menu
//! item. Each callback has access to the application data, and also gets access to a
//! [`MenuEventCtx`], which allows for submitting [`Command`]s.
//!
//! ## Refreshing and rebuilding
//!
//! Menus, like widgets, update themselves based on changes in the data. There are two different
//! ways that the menus update themselves:
//!
//! - a "refresh" is when the menu items update their text or their status (e.g. disabled,
//!   selected) based on changes to the data. Menu refreshes are handled for you automatically. For
//!   example, if you create a menu item whose title is a [`LabelText::Dynamic`] then that title
//!   will be kept up-to-date for you.
//!
//!   The limitation of a "refresh" is that it cannot change the structure of the menus (e.g. by
//!   adding new items or moving things around).
//!
//! - a "rebuild" is when the menu is rebuilt from scratch. When you first set a menu (e.g. using
//!   [`WindowDesc::menu`]), you provide a callback for building the menu from data; a rebuild is
//!   when the menu decides to rebuild itself by invoking that callback again.
//!
//!   Rebuilds have none of the limitations of refreshes, but Druid does not automatically decide
//!   when to do them. You need to use [`Menu::rebuild_on`] to decide when rebuild should
//!   occur.
//!
//! ## The macOS app menu
//!
//! On macOS, the main menu belongs to the application, not to the window.
//!
//! In Druid, whichever window is frontmost will have its menu displayed as the application menu.
//!
//! ## Examples
//!
//! Creating the default app menu for macOS:
//!
//! ```
//! use druid::commands;
//! use druid::{Data, LocalizedString, Menu, MenuItem, SysMods};
//!
//! fn macos_application_menu<T: Data>() -> Menu<T> {
//!     Menu::new(LocalizedString::new("macos-menu-application-menu"))
//!         .entry(
//!             MenuItem::new(LocalizedString::new("macos-menu-about-app"))
//!                 // You need to handle the SHOW_ABOUT command yourself (or else do something
//!                 // directly to the data here instead of using a command).
//!                 .command(commands::SHOW_ABOUT),
//!         )
//!         .separator()
//!         .entry(
//!             MenuItem::new(LocalizedString::new("macos-menu-preferences"))
//!                 // You need to handle the SHOW_PREFERENCES command yourself (or else do something
//!                 // directly to the data here instead of using a command).
//!                 .command(commands::SHOW_PREFERENCES)
//!                 .hotkey(SysMods::Cmd, ","),
//!         )
//!         .separator()
//!         .entry(MenuItem::new(LocalizedString::new("macos-menu-services")))
//!         .entry(
//!             MenuItem::new(LocalizedString::new("macos-menu-hide-app"))
//!                 // druid handles the HIDE_APPLICATION command automatically
//!                 .command(commands::HIDE_APPLICATION)
//!                 .hotkey(SysMods::Cmd, "h"),
//!         )
//!         .entry(
//!             MenuItem::new(LocalizedString::new("macos-menu-hide-others"))
//!                 // druid handles the HIDE_OTHERS command automatically
//!                 .command(commands::HIDE_OTHERS)
//!                 .hotkey(SysMods::AltCmd, "h"),
//!         )
//!         .entry(
//!             MenuItem::new(LocalizedString::new("macos-menu-show-all"))
//!                 // You need to handle the SHOW_ALL command yourself (or else do something
//!                 // directly to the data here instead of using a command).
//!                 .command(commands::SHOW_ALL)
//!         )
//!         .separator()
//!         .entry(
//!             MenuItem::new(LocalizedString::new("macos-menu-quit-app"))
//!                 // druid handles the QUIT_APP command automatically
//!                 .command(commands::QUIT_APP)
//!                 .hotkey(SysMods::Cmd, "q"),
//!         )
//! }
//! ```
//!
//! [`LabelText::Dynamic`]: crate::widget::LabelText::Dynamic
//! [`WindowDesc::menu`]: crate::WindowDesc::menu
//! [`Command`]: crate::Command

use std::num::NonZeroU32;

use crate::core::CommandQueue;
use crate::kurbo::Point;
use crate::shell::{Counter, HotKey, IntoKey, Menu as PlatformMenu};
use crate::widget::LabelText;
use crate::{ArcStr, Command, Data, Env, Lens, RawMods, Target, WindowId};

static COUNTER: Counter = Counter::new();

pub mod sys;

type MenuBuild<T> = Box<dyn FnMut(Option<WindowId>, &T, &Env) -> Menu<T>>;

/// This is for completely recreating the menus (for when you want to change the actual menu
/// structure, rather than just, say, enabling or disabling entries).
pub(crate) struct MenuManager<T> {
    // The function for rebuilding the menu. If this is `None` (which is the case for context
    // menus), `menu` will always be `Some(..)`.
    build: Option<MenuBuild<T>>,
    popup: bool,
    old_data: Option<T>,
    menu: Option<Menu<T>>,
}

/// A menu displayed as a pop-over.
pub(crate) struct ContextMenu<T> {
    pub(crate) menu: Menu<T>,
    pub(crate) location: Point,
}

impl<T: Data> MenuManager<T> {
    /// Create a new [`MenuManager`] for a title-bar menu.
    pub fn new(
        build: impl FnMut(Option<WindowId>, &T, &Env) -> Menu<T> + 'static,
    ) -> MenuManager<T> {
        MenuManager {
            build: Some(Box::new(build)),
            popup: false,
            old_data: None,
            menu: None,
        }
    }

    /// Create a new [`MenuManager`] for a context menu.
    pub fn new_for_popup(menu: Menu<T>) -> MenuManager<T> {
        MenuManager {
            build: None,
            popup: true,
            old_data: None,
            menu: Some(menu),
        }
    }

    /// If this platform always expects windows to have a menu by default, returns a menu.
    /// Otherwise, returns `None`.
    #[allow(unreachable_code)]
    pub fn platform_default() -> Option<MenuManager<T>> {
        #[cfg(target_os = "macos")]
        return Some(MenuManager::new(|_, _, _| sys::mac::application::default()));

        #[cfg(any(target_os = "windows", target_os = "linux"))]
        return None;

        // we want to explicitly handle all platforms; log if a platform is missing.
        tracing::warn!("MenuManager::platform_default is not implemented for this platform.");
        None
    }

    /// Called when a menu event is received from the system.
    pub fn event(
        &mut self,
        queue: &mut CommandQueue,
        window: Option<WindowId>,
        id: MenuItemId,
        data: &mut T,
        env: &Env,
    ) {
        if let Some(m) = &mut self.menu {
            let mut ctx = MenuEventCtx { queue, window };
            m.activate(&mut ctx, id, data, env);
        }
    }

    /// Build an initial menu from the application data.
    pub fn initialize(&mut self, window: Option<WindowId>, data: &T, env: &Env) -> PlatformMenu {
        if let Some(build) = &mut self.build {
            self.menu = Some((build)(window, data, env));
        }
        self.old_data = Some(data.clone());
        self.refresh(data, env)
    }

    /// Update the menu based on a change to the data.
    ///
    /// Returns a new `PlatformMenu` if the menu has changed; returns `None` if it hasn't.
    pub fn update(
        &mut self,
        window: Option<WindowId>,
        data: &T,
        env: &Env,
    ) -> Option<PlatformMenu> {
        if let (Some(menu), Some(old_data)) = (self.menu.as_mut(), self.old_data.as_ref()) {
            let ret = match menu.update(old_data, data, env) {
                MenuUpdate::NeedsRebuild => {
                    if let Some(build) = &mut self.build {
                        self.menu = Some((build)(window, data, env));
                    } else {
                        tracing::warn!("tried to rebuild a context menu");
                    }
                    Some(self.refresh(data, env))
                }
                MenuUpdate::NeedsRefresh => Some(self.refresh(data, env)),
                MenuUpdate::UpToDate => None,
            };
            self.old_data = Some(data.clone());
            ret
        } else {
            tracing::error!("tried to update uninitialized menus");
            None
        }
    }

    /// Builds a new menu for displaying the given data.
    ///
    /// Mostly you should probably use `update` instead, because that actually checks whether a
    /// refresh is necessary.
    pub fn refresh(&mut self, data: &T, env: &Env) -> PlatformMenu {
        if let Some(menu) = self.menu.as_mut() {
            let mut ctx = MenuBuildCtx::new(self.popup);
            menu.refresh_children(&mut ctx, data, env);
            ctx.current
        } else {
            tracing::error!("tried to refresh uninitialized menus");
            PlatformMenu::new()
        }
    }
}

/// This context is available to the callback that is called when a menu item is activated.
///
/// Currently, it only allows for submission of [`Command`]s.
///
/// [`Command`]: crate::Command
pub struct MenuEventCtx<'a> {
    window: Option<WindowId>,
    queue: &'a mut CommandQueue,
}

/// This context helps menu items to build the platform menu.
struct MenuBuildCtx {
    current: PlatformMenu,
}

impl MenuBuildCtx {
    fn new(popup: bool) -> MenuBuildCtx {
        MenuBuildCtx {
            current: if popup {
                PlatformMenu::new_for_popup()
            } else {
                PlatformMenu::new()
            },
        }
    }

    fn with_submenu(&mut self, text: &str, enabled: bool, f: impl FnOnce(&mut MenuBuildCtx)) {
        let mut child = MenuBuildCtx::new(false);
        f(&mut child);
        self.current.add_dropdown(child.current, text, enabled);
    }

    fn add_item(
        &mut self,
        id: u32,
        text: &str,
        key: Option<&HotKey>,
        enabled: bool,
        selected: bool,
    ) {
        self.current.add_item(id, text, key, enabled, selected);
    }

    fn add_separator(&mut self) {
        self.current.add_separator();
    }
}

impl<'a> MenuEventCtx<'a> {
    /// Submit a [`Command`] to be handled by the main widget tree.
    ///
    /// If the command's target is [`Target::Auto`], it will be sent to the menu's window if the
    /// menu is associated with a window, or to [`Target::Global`] if the menu is not associated
    /// with a window.
    ///
    /// See [`EventCtx::submit_command`] for more information.
    ///
    /// [`Command`]: crate::Command
    /// [`EventCtx::submit_command`]: crate::EventCtx::submit_command
    /// [`Target::Auto`]: crate::Target::Auto
    /// [`Target::Global`]: crate::Target::Global
    pub fn submit_command(&mut self, cmd: impl Into<Command>) {
        self.queue.push_back(
            cmd.into()
                .default_to(self.window.map(Target::Window).unwrap_or(Target::Global)),
        );
    }
}

#[derive(Clone, Copy, Debug)]
enum MenuUpdate {
    /// The structure of the current menu is ok, but some elements need to be refreshed (e.g.
    /// changing their text, whether they are enabled, etc.)
    NeedsRefresh,
    /// The structure of the menu has changed; we need to rebuilt from scratch.
    NeedsRebuild,
    /// No need to rebuild anything.
    UpToDate,
}

impl MenuUpdate {
    fn combine(self, other: MenuUpdate) -> MenuUpdate {
        use MenuUpdate::*;
        match (self, other) {
            (NeedsRebuild, _) | (_, NeedsRebuild) => NeedsRebuild,
            (NeedsRefresh, _) | (_, NeedsRefresh) => NeedsRefresh,
            _ => UpToDate,
        }
    }
}

/// This is the trait that enables recursive visiting of all menu entries. It isn't publically
/// visible (the publically visible analogue of this is `Into<MenuEntry<T>>`).
trait MenuVisitor<T> {
    /// Called when a menu item is activated.
    ///
    /// `id` is the id of the entry that got activated. If this is different from your id, you are
    /// responsible for routing the activation to your child items.
    fn activate(&mut self, ctx: &mut MenuEventCtx, id: MenuItemId, data: &mut T, env: &Env);

    /// Called when the data is changed.
    fn update(&mut self, old_data: &T, data: &T, env: &Env) -> MenuUpdate;

    /// Called to refresh the menu.
    fn refresh(&mut self, ctx: &mut MenuBuildCtx, data: &T, env: &Env);
}

/// A wrapper for a menu item (or submenu) to give it access to a part of its parent data.
///
/// This is the menu analogue of [`LensWrap`]. You will usually create it with [`Menu::lens`] or
/// [`MenuItem::lens`] instead of using this struct directly.
///
/// [`LensWrap`]: crate::widget::LensWrap
pub struct MenuLensWrap<L, U> {
    lens: L,
    inner: Box<dyn MenuVisitor<U>>,
    old_data: Option<U>,
    old_env: Option<Env>,
}

impl<T: Data, U: Data, L: Lens<T, U>> MenuVisitor<T> for MenuLensWrap<L, U> {
    fn activate(&mut self, ctx: &mut MenuEventCtx, id: MenuItemId, data: &mut T, env: &Env) {
        let inner = &mut self.inner;
        self.lens
            .with_mut(data, |u| inner.activate(ctx, id, u, env));
    }

    fn update(&mut self, old_data: &T, data: &T, env: &Env) -> MenuUpdate {
        let inner = &mut self.inner;
        let lens = &self.lens;
        let cached_old_data = &mut self.old_data;
        let cached_old_env = &mut self.old_env;
        lens.with(old_data, |old| {
            lens.with(data, |new| {
                let ret = if cached_old_data.as_ref().map(|x| x.same(old)) == Some(true)
                    && cached_old_env.as_ref().map(|x| x.same(env)) == Some(true)
                {
                    MenuUpdate::UpToDate
                } else {
                    inner.update(old, new, env)
                };
                *cached_old_data = Some(new.clone());
                *cached_old_env = Some(env.clone());
                ret
            })
        })
    }

    fn refresh(&mut self, ctx: &mut MenuBuildCtx, data: &T, env: &Env) {
        let inner = &mut self.inner;
        self.lens.with(data, |u| inner.refresh(ctx, u, env))
    }
}

impl<T: Data, U: Data, L: Lens<T, U> + 'static> From<MenuLensWrap<L, U>> for MenuEntry<T> {
    fn from(m: MenuLensWrap<L, U>) -> MenuEntry<T> {
        MenuEntry { inner: Box::new(m) }
    }
}

/// An entry in a menu.
///
/// An entry is either a [`MenuItem`], a submenu (i.e. [`Menu`]), or one of a few other
/// possibilities (such as one of the two options above, wrapped in a [`MenuLensWrap`]).
pub struct MenuEntry<T> {
    inner: Box<dyn MenuVisitor<T>>,
}

type MenuPredicate<T> = Box<dyn FnMut(&T, &T, &Env) -> bool>;

/// A menu.
///
/// Menus can be nested arbitrarily, so this could also be a submenu.
/// See the [module level documentation](crate::menu) for more on how to use menus.
pub struct Menu<T> {
    rebuild_on: Option<MenuPredicate<T>>,
    refresh_on: Option<MenuPredicate<T>>,
    item: MenuItem<T>,
    children: Vec<MenuEntry<T>>,
    // bloom?
}

#[doc(hidden)]
#[deprecated(since = "0.8.0", note = "Renamed to Menu")]
pub type MenuDesc<T> = Menu<T>;

impl<T: Data> From<Menu<T>> for MenuEntry<T> {
    fn from(menu: Menu<T>) -> MenuEntry<T> {
        MenuEntry {
            inner: Box::new(menu),
        }
    }
}

type MenuCallback<T> = Box<dyn FnMut(&mut MenuEventCtx, &mut T, &Env)>;
type HotKeyCallback<T> = Box<dyn FnMut(&T, &Env) -> Option<HotKey>>;

/// An item in a menu.
///
/// See the [module level documentation](crate::menu) for more on how to use menus.
pub struct MenuItem<T> {
    id: MenuItemId,

    title: LabelText<T>,
    callback: Option<MenuCallback<T>>,
    hotkey: Option<HotKeyCallback<T>>,
    selected: Option<Box<dyn FnMut(&T, &Env) -> bool>>,
    enabled: Option<Box<dyn FnMut(&T, &Env) -> bool>>,

    // The last resolved state of this menu item. This is basically consists of all the properties
    // above, but "static" versions of them not depending on the data.
    old_state: Option<MenuItemState>,
}

impl<T: Data> From<MenuItem<T>> for MenuEntry<T> {
    fn from(i: MenuItem<T>) -> MenuEntry<T> {
        MenuEntry { inner: Box::new(i) }
    }
}

struct Separator;

impl<T: Data> From<Separator> for MenuEntry<T> {
    fn from(s: Separator) -> MenuEntry<T> {
        MenuEntry { inner: Box::new(s) }
    }
}

impl<T: Data> Menu<T> {
    /// Create an empty menu.
    pub fn empty() -> Menu<T> {
        Menu {
            rebuild_on: None,
            refresh_on: None,
            item: MenuItem::new(""),
            children: Vec::new(),
        }
    }

    /// Create a menu with the given name.
    pub fn new(title: impl Into<LabelText<T>>) -> Menu<T> {
        Menu {
            rebuild_on: None,
            refresh_on: None,
            item: MenuItem::new(title),
            children: Vec::new(),
        }
    }

    /// Provide a callback for determining whether this item should be enabled.
    ///
    /// Whenever the callback returns `true`, the menu will be enabled.
    pub fn enabled_if(mut self, enabled: impl FnMut(&T, &Env) -> bool + 'static) -> Self {
        self.item = self.item.enabled_if(enabled);
        self
    }

    /// Enable or disable this menu.
    pub fn enabled(self, enabled: bool) -> Self {
        self.enabled_if(move |_data, _env| enabled)
    }

    #[doc(hidden)]
    #[deprecated(since = "0.8.0", note = "use entry instead")]
    pub fn append_entry(self, entry: impl Into<MenuEntry<T>>) -> Self {
        self.entry(entry)
    }

    #[doc(hidden)]
    #[deprecated(since = "0.8.0", note = "use entry instead")]
    pub fn append_separator(self) -> Self {
        self.separator()
    }

    /// Append a menu entry to this menu, returning the modified menu.
    pub fn entry(mut self, entry: impl Into<MenuEntry<T>>) -> Self {
        self.children.push(entry.into());
        self
    }

    /// Append a separator to this menu, returning the modified menu.
    pub fn separator(self) -> Self {
        self.entry(Separator)
    }

    /// Supply a function to check when this menu needs to refresh itself.
    ///
    /// The arguments to the callback are (in order):
    /// - the previous value of the data,
    /// - the current value of the data, and
    /// - the current value of the environment.
    ///
    /// The callback should return true if the menu needs to refresh itself.
    ///
    /// This callback is intended to be purely an optimization. If you do create a menu without
    /// supplying a refresh callback, the menu will recursively check whether any children have
    /// changed and refresh itself if any have. By supplying a callback here, you can short-circuit
    /// those recursive calls.
    pub fn refresh_on(mut self, refresh: impl FnMut(&T, &T, &Env) -> bool + 'static) -> Self {
        self.refresh_on = Some(Box::new(refresh));
        self
    }

    /// Supply a function to check when this menu needs to be rebuild from scratch.
    ///
    /// The arguments to the callback are (in order):
    /// - the previous value of the data,
    /// - the current value of the data, and
    /// - the current value of the environment.
    ///
    /// The callback should return true if the menu needs to be rebuilt.
    ///
    /// The difference between rebuilding and refreshing (as in [`refresh_on`]) is
    /// that rebuilding creates the menu from scratch using the original menu-building callback,
    /// whereas refreshing involves tweaking the existing menu entries (e.g. enabling or disabling
    /// items).
    ///
    /// If you do not provide a callback using this method, the menu will never get rebuilt.  Also,
    /// only window and application menus get rebuilt; context menus never do.
    ///
    /// [`refresh_on`]: self::Menu<T>::refresh_on
    pub fn rebuild_on(mut self, rebuild: impl FnMut(&T, &T, &Env) -> bool + 'static) -> Self {
        self.rebuild_on = Some(Box::new(rebuild));
        self
    }

    /// Wraps this menu in a lens, so that it can be added to a `Menu<S>`.
    pub fn lens<S: Data>(self, lens: impl Lens<S, T> + 'static) -> MenuEntry<S> {
        MenuLensWrap {
            lens,
            inner: Box::new(self),
            old_data: None,
            old_env: None,
        }
        .into()
    }

    // This is like MenuVisitor::refresh, but it doesn't add a submenu for the current level.
    // (This is the behavior we need for the top-level (unnamed) menu, which contains (e.g.) File,
    // Edit, etc. as submenus.)
    fn refresh_children(&mut self, ctx: &mut MenuBuildCtx, data: &T, env: &Env) {
        self.item.resolve(data, env);
        for child in &mut self.children {
            child.refresh(ctx, data, env);
        }
    }
}

impl<T: Data> MenuItem<T> {
    /// Create a new menu item with a given name.
    pub fn new(title: impl Into<LabelText<T>>) -> MenuItem<T> {
        let mut id = COUNTER.next() as u32;
        if id == 0 {
            id = COUNTER.next() as u32;
        }
        MenuItem {
            id: MenuItemId(std::num::NonZeroU32::new(id)),
            title: title.into(),
            callback: None,
            hotkey: None,
            selected: None,
            enabled: None,
            old_state: None,
        }
    }

    /// Provide a callback that will be invoked when this menu item is chosen.
    pub fn on_activate(
        mut self,
        on_activate: impl FnMut(&mut MenuEventCtx, &mut T, &Env) + 'static,
    ) -> Self {
        self.callback = Some(Box::new(on_activate));
        self
    }

    /// Provide a [`Command`] that will be sent when this menu item is chosen.
    ///
    /// This is equivalent to `self.on_activate(move |ctx, _data, _env| ctx.submit_command(cmd))`.
    /// If the command's target is [`Target::Auto`], it will be sent to the menu's window if the
    /// menu is associated with a window, or to [`Target::Global`] if the menu is not associated
    /// with a window.
    ///
    /// [`Command`]: crate::Command
    /// [`Target::Auto`]: crate::Target::Auto
    /// [`Target::Global`]: crate::Target::Global
    pub fn command(self, cmd: impl Into<Command>) -> Self {
        let cmd = cmd.into();
        self.on_activate(move |ctx, _data, _env| ctx.submit_command(cmd.clone()))
    }

    /// Provide a hotkey for activating this menu item.
    ///
    /// This is equivalent to
    /// `self.dynamic_hotkey(move |_, _| Some(HotKey::new(mods, key))`
    pub fn hotkey(self, mods: impl Into<Option<RawMods>>, key: impl IntoKey) -> Self {
        let hotkey = HotKey::new(mods, key);
        self.dynamic_hotkey(move |_, _| Some(hotkey.clone()))
    }

    /// Provide a dynamic hotkey for activating this menu item.
    ///
    /// The hotkey can change depending on the data.
    pub fn dynamic_hotkey(
        mut self,
        hotkey: impl FnMut(&T, &Env) -> Option<HotKey> + 'static,
    ) -> Self {
        self.hotkey = Some(Box::new(hotkey));
        self
    }

    /// Provide a callback for determining whether this menu item should be enabled.
    ///
    /// Whenever the callback returns `true`, the item will be enabled.
    pub fn enabled_if(mut self, enabled: impl FnMut(&T, &Env) -> bool + 'static) -> Self {
        self.enabled = Some(Box::new(enabled));
        self
    }

    /// Enable or disable this menu item.
    pub fn enabled(self, enabled: bool) -> Self {
        self.enabled_if(move |_data, _env| enabled)
    }

    /// Provide a callback for determining whether this menu item should be selected.
    ///
    /// Whenever the callback returns `true`, the item will be selected.
    pub fn selected_if(mut self, selected: impl FnMut(&T, &Env) -> bool + 'static) -> Self {
        self.selected = Some(Box::new(selected));
        self
    }

    /// Select or deselect this menu item.
    pub fn selected(self, selected: bool) -> Self {
        self.selected_if(move |_data, _env| selected)
    }

    /// Wraps this menu item in a lens, so that it can be added to a `Menu<S>`.
    pub fn lens<S: Data>(self, lens: impl Lens<S, T> + 'static) -> MenuEntry<S> {
        MenuLensWrap {
            lens,
            inner: Box::new(self),
            old_data: None,
            old_env: None,
        }
        .into()
    }

    fn resolve(&mut self, data: &T, env: &Env) -> bool {
        self.title.resolve(data, env);
        let new_state = MenuItemState {
            title: self.title.display_text(),
            hotkey: self.hotkey.as_mut().and_then(|h| h(data, env)),
            selected: self
                .selected
                .as_mut()
                .map(|s| s(data, env))
                .unwrap_or(false),
            enabled: self.enabled.as_mut().map(|e| e(data, env)).unwrap_or(true),
        };
        let ret = self.old_state.as_ref() != Some(&new_state);
        self.old_state = Some(new_state);
        ret
    }

    // Panics if we haven't been resolved.
    fn text(&self) -> &str {
        &self.old_state.as_ref().unwrap().title
    }

    // Panics if we haven't been resolved.
    fn is_enabled(&self) -> bool {
        self.old_state.as_ref().unwrap().enabled
    }
}

impl<T: Data> MenuVisitor<T> for Menu<T> {
    fn activate(&mut self, ctx: &mut MenuEventCtx, id: MenuItemId, data: &mut T, env: &Env) {
        for child in &mut self.children {
            child.activate(ctx, id, data, env);
        }
    }

    fn update(&mut self, old_data: &T, data: &T, env: &Env) -> MenuUpdate {
        if let Some(rebuild_on) = &mut self.rebuild_on {
            if rebuild_on(old_data, data, env) {
                return MenuUpdate::NeedsRebuild;
            }
        }
        if let Some(refresh_on) = &mut self.refresh_on {
            if refresh_on(old_data, data, env) {
                return MenuUpdate::NeedsRefresh;
            }
        }

        let mut ret = self.item.update(old_data, data, env);
        for child in &mut self.children {
            ret = ret.combine(child.update(old_data, data, env));
        }
        ret
    }

    fn refresh(&mut self, ctx: &mut MenuBuildCtx, data: &T, env: &Env) {
        self.item.resolve(data, env);
        let children = &mut self.children;
        ctx.with_submenu(self.item.text(), self.item.is_enabled(), |ctx| {
            for child in children {
                child.refresh(ctx, data, env);
            }
        });
    }
}

impl<T: Data> MenuVisitor<T> for MenuEntry<T> {
    fn activate(&mut self, ctx: &mut MenuEventCtx, id: MenuItemId, data: &mut T, env: &Env) {
        self.inner.activate(ctx, id, data, env);
    }

    fn update(&mut self, old_data: &T, data: &T, env: &Env) -> MenuUpdate {
        self.inner.update(old_data, data, env)
    }

    fn refresh(&mut self, ctx: &mut MenuBuildCtx, data: &T, env: &Env) {
        self.inner.refresh(ctx, data, env);
    }
}

impl<T: Data> MenuVisitor<T> for MenuItem<T> {
    fn activate(&mut self, ctx: &mut MenuEventCtx, id: MenuItemId, data: &mut T, env: &Env) {
        if id == self.id {
            if let Some(callback) = &mut self.callback {
                callback(ctx, data, env);
            }
        }
    }

    fn update(&mut self, _old_data: &T, data: &T, env: &Env) -> MenuUpdate {
        if self.resolve(data, env) {
            MenuUpdate::NeedsRefresh
        } else {
            MenuUpdate::UpToDate
        }
    }

    fn refresh(&mut self, ctx: &mut MenuBuildCtx, data: &T, env: &Env) {
        self.resolve(data, env);
        let state = self.old_state.as_ref().unwrap();
        ctx.add_item(
            self.id.0.map(|x| x.get()).unwrap_or(0),
            &state.title,
            state.hotkey.as_ref(),
            state.enabled,
            state.selected,
        );
    }
}

impl<T: Data> MenuVisitor<T> for Separator {
    fn activate(&mut self, _ctx: &mut MenuEventCtx, _id: MenuItemId, _data: &mut T, _env: &Env) {}

    fn update(&mut self, _old_data: &T, _data: &T, _env: &Env) -> MenuUpdate {
        MenuUpdate::UpToDate
    }
    fn refresh(&mut self, ctx: &mut MenuBuildCtx, _data: &T, _env: &Env) {
        ctx.add_separator();
    }
}

// The resolved state of a menu item.
#[derive(PartialEq)]
struct MenuItemState {
    title: ArcStr,
    hotkey: Option<HotKey>,
    selected: bool,
    enabled: bool,
}

/// Uniquely identifies a menu item.
///
/// On the druid-shell side, the id is represented as a u32.
/// We reserve '0' as a placeholder value; on the Rust side
/// we represent this as an `Option<NonZerou32>`, which better
/// represents the semantics of our program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MenuItemId(Option<NonZeroU32>);

impl MenuItemId {
    pub(crate) fn new(id: u32) -> MenuItemId {
        MenuItemId(NonZeroU32::new(id))
    }
}
