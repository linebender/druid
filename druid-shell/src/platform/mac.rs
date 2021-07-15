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
