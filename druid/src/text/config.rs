// Copyright 2017 The xi-editor Authors.
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

/// The concrete type for buffer-related settings.
#[derive(Debug, Clone, PartialEq)]
pub struct BufferItems {
    pub line_ending: String,
    pub tab_size: usize,
    pub translate_tabs_to_spaces: bool,
    pub use_tab_stops: bool,
    pub surrounding_pairs: Vec<(String, String)>,
}

pub const DEFAULT_CONFIG: &'static BufferItems = &BufferItems {
    line_ending: String::new(),
    tab_size: 4,
    translate_tabs_to_spaces: false,
    use_tab_stops: false,
    surrounding_pairs: Vec::new(),
};
