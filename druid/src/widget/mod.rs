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

mod align;
pub use crate::widget::align::Align;

mod button;
pub use crate::widget::button::Button;

mod label;
pub use crate::widget::label::{DynLabel, Label, LabelText};

mod either;
pub use crate::widget::either::Either;

mod flex;
pub use crate::widget::flex::{Column, Flex, Row};

mod padding;
pub use crate::widget::padding::Padding;

mod scroll;
pub use crate::widget::scroll::Scroll;

mod progress_bar;
pub use crate::widget::progress_bar::ProgressBar;

mod slider;
pub use crate::widget::slider::Slider;

mod textbox;
pub use crate::widget::textbox::TextBox;

mod sized_box;
pub use crate::widget::sized_box::SizedBox;

mod checkbox;
pub use crate::widget::checkbox::Checkbox;

mod radio;
pub use crate::widget::radio::{Radio, RadioGroup};

mod container;
pub use crate::widget::container::Container;

mod split;
pub use crate::widget::split::Split;

mod switch;
pub use crate::widget::switch::Switch;

mod env_scope;
pub use crate::widget::env_scope::EnvScope;

mod widget_ext;
pub use widget_ext::WidgetExt;

mod list;
pub use crate::widget::list::{List, ListIter};
