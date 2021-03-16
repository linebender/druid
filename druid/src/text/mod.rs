// Copyright 2018 The Druid Authors.
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

//! Text editing utilities.

mod attribute;
pub mod backspace;
mod editable_text;
mod font_descriptor;
pub mod format;
mod input_component;
mod input_methods;
mod layout;
pub mod movement;
mod rich_text;
pub mod selection;
mod storage;

pub use self::attribute::{Attribute, AttributeSpans};
pub use self::backspace::offset_for_delete_backwards;
pub use self::editable_text::{EditableText, EditableTextCursor, StringCursor};
pub use self::font_descriptor::FontDescriptor;
pub use self::layout::{LayoutMetrics, TextLayout};
pub use self::movement::{movement, Movement};
pub use self::selection::Selection;
pub use input_component::{EditSession, TextComponent};
pub use input_methods::ImeHandlerRef;
pub use rich_text::{AttributesAdder, RichText, RichTextBuilder};
pub use storage::{ArcStr, TextStorage};

pub(crate) use input_methods::TextFieldRegistration;
