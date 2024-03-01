// Copyright 2021 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! macOS specific extensions.

/// macOS specific extensions to [`Application`]
///
/// [`Application`]: crate::Application
pub trait ApplicationExt {
    /// Hide the application this window belongs to. (cmd+H)
    fn hide(&self);

    /// Hide all other applications. (cmd+opt+H)
    fn hide_others(&self);

    /// Sets the global application menu, on platforms where there is one.
    ///
    /// On platforms with no global application menu, this has no effect.
    fn set_menu(&self, menu: crate::Menu);
}

#[cfg(test)]
mod test {
    use crate::Application;

    use super::*;
    use static_assertions as sa;
    sa::assert_impl_all!(Application: ApplicationExt);
}
