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

//! Core types and mechanisms for the widget hierarchy.
//!
//! //! Note: the organization of this code roughly follows the existing Druid
//! widget system, particularly its core.rs.

use bitflags::bitflags;
use druid_shell::kurbo::{Point, Size};

use super::AnyWidget;

bitflags! {
    #[derive(Default)]
    pub(crate) struct PodFlags: u32 {
        const REQUEST_UPDATE = 1;
        const REQUEST_LAYOUT = 2;
        const REQUEST_PAINT = 4;

        const UPWARD_FLAGS = Self::REQUEST_LAYOUT.bits | Self::REQUEST_PAINT.bits;
        const INIT_FLAGS = Self::REQUEST_UPDATE.bits | Self::REQUEST_LAYOUT.bits | Self::REQUEST_PAINT.bits;
    }
}

/// A pod that contains a widget (in a container).
pub struct Pod {
    pub(crate) state: WidgetState,
    pub(crate) widget: Box<dyn AnyWidget>,
}

#[derive(Default, Debug)]
pub(crate) struct WidgetState {
    pub(crate) flags: PodFlags,
    pub(crate) origin: Point,
    /// The minimum intrinsic size of the widget.
    pub(crate) min_size: Size,
    /// The maximum intrinsic size of the widget.
    pub(crate) max_size: Size,
    /// The size proposed by the widget's container.
    pub(crate) proposed_size: Size,
    /// The size of the widget.
    pub(crate) size: Size,
}
