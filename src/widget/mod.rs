// Copyright 2018 The xi-editor Authors.
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

//! Common widgets.

mod button;
pub use crate::widget::button::{Button, Label};

mod flex;
pub use crate::widget::flex::{Column, Flex, Row};

mod padding;
pub use crate::widget::padding::Padding;

/*

// The widget trait should probably at least get its own file. When it does,
// the following methods should probably go into it:

#[derive(Debug, Clone)]
pub enum KeyVariant {
    /// A virtual-key code, same as WM_KEYDOWN message.
    Vkey(i32),
    /// A Unicode character.
    Char(char),
}

*/
