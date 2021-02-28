// Copyright 2019 The Druid Authors.
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

//! Common types for representing touch events

use crate::kurbo::Point;
use crate::Modifiers;
use crate::platform;

#[derive(Debug, Clone, PartialEq)]
pub struct TouchEvent {
    pub pos: Point,
    pub mods: Modifiers,
    pub focus: bool,
    pub sequence_id: Option<TouchSequenceId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TouchSequenceId {
    Value(platform::window::TouchSequenceId)
}
