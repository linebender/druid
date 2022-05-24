// Copyright 2022 The Druid Authors.
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

//! Prototype implementation of Xilem architecture.
//!
//! This is a skeletal, proof-of-concept UI toolkit to prove out the Xilem
//! architectural ideas.

mod app;
mod app_main;
mod event;
mod id;
mod view;
mod view_seq;
mod widget;

pub use app::App;
pub use app_main::AppLauncher;
pub use view::adapt::Adapt;
pub use view::button::button;
pub use view::layout_observer::LayoutObserver;
pub use view::list::list;
pub use view::memoize::Memoize;
pub use view::scroll_view::scroll_view;
pub use view::vstack::v_stack;
pub use view::View;
pub use widget::align::{AlignmentAxis, AlignmentProxy, HorizAlignment, VertAlignment};
pub use widget::Widget;
