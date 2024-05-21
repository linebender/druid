// Copyright 2021 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Linux specific extensions.
use crate::Clipboard;

/// Linux specific extensions to [`Application`]
///
/// [`Application`]: crate::Application
pub trait ApplicationExt {
    /// Returns a handle to the primary system clipboard.
    ///
    /// This is useful for middle mouse paste.
    fn primary_clipboard(&self) -> Clipboard;
}

#[cfg(test)]
#[allow(unused_imports)]
mod test {
    use crate::Application;

    use super::*;
    use static_assertions as sa;
    // TODO: impl ApplicationExt for wayland
    #[cfg(not(feature = "wayland"))]
    sa::assert_impl_all!(Application: ApplicationExt);
}
