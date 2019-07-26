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

mod action_wrapper;
pub use crate::widget::action_wrapper::ActionWrapper;

mod button;
pub use crate::widget::button::{Button, DynLabel, Label};

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
