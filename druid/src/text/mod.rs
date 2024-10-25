// Copyright 2018 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Editing and displaying text.

mod attribute;
mod backspace;
mod editable_text;
mod font_descriptor;
mod format;
mod input_component;
mod input_methods;
mod layout;
mod movement;
mod rich_text;
mod storage;

pub use crate::piet::{FontFamily, FontStyle, FontWeight, TextAlignment};
pub use druid_shell::text::{
    Action as TextAction, Affinity, Direction, Event as ImeInvalidation, InputHandler, Movement,
    Selection, VerticalMovement, WritingDirection,
};

pub use self::attribute::{Attribute, AttributeSpans, Link};
pub use self::backspace::offset_for_delete_backwards;
pub use self::editable_text::{EditableText, EditableTextCursor, StringCursor};
pub use self::font_descriptor::FontDescriptor;
pub use self::format::{Formatter, ParseFormatter, Validation, ValidationError};
pub use self::layout::{LayoutMetrics, TextLayout};
pub use self::movement::movement;
pub use input_component::{EditSession, TextComponent};
pub use input_methods::ImeHandlerRef;
pub use rich_text::{AttributesAdder, RichText, RichTextBuilder};
pub use storage::{ArcStr, EnvUpdateCtx, TextStorage};

pub(crate) use input_methods::TextFieldRegistration;
