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

//! A data structure for representing widget trees.

use std::collections::HashMap;

/// A description widget and its children, clonable and comparable, meant
/// for testing and debugging. This is extremely not optimized.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct DebugState {
    /// The widget's type as a human-readable string.
    pub display_name: String,
    /// If a widget has a "central" value (for instance, a textbox's contents),
    /// it is stored here.
    pub main_value: String,
    /// Untyped values that reveal useful information about the widget.
    pub other_values: HashMap<String, String>,
    /// Debug info of child widgets.
    pub children: Vec<DebugState>,
}

impl std::fmt::Debug for DebugState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.other_values.is_empty() && self.children.is_empty() && self.main_value.is_empty() {
            f.write_str(&self.display_name)
        } else if self.other_values.is_empty() && self.children.is_empty() {
            f.debug_tuple(&self.display_name)
                .field(&self.main_value)
                .finish()
        } else if self.other_values.is_empty() && self.main_value.is_empty() {
            let mut f_tuple = f.debug_tuple(&self.display_name);
            for child in &self.children {
                f_tuple.field(child);
            }
            f_tuple.finish()
        } else {
            let mut f_struct = f.debug_struct(&self.display_name);
            if !self.main_value.is_empty() {
                f_struct.field("_main_value_", &self.main_value);
            }
            for (key, value) in self.other_values.iter() {
                f_struct.field(key, &value);
            }
            if !self.children.is_empty() {
                f_struct.field("children", &self.children);
            }
            f_struct.finish()
        }
    }
}
