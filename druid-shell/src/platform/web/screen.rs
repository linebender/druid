// Copyright 2020 The druid Authors.
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

//! Monitor and Screen information ignored for web.

use crate::screen::Monitor;
use crate::kurbo::Size;

pub fn get_display_size() -> Size {
    //Ignore?
    Size::new(0.0, 0.0)
}

pub fn get_monitors() -> Vec<Monitor> {
    //Ignore?
    Vec::<Monitor>::new()
}
