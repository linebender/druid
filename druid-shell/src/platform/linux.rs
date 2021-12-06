// Copyright 2021 The Druid Authors.
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
