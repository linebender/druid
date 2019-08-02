// Copyright 2019 The xi-editor Authors.
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

//! An initial theme

use std::collections::HashMap;

use crate::piet::Color;

use crate::EnvValue;

pub fn init_theme() -> EnvValue {
    let mut m = HashMap::new();
    // TODO: Consider EnvMap (newtype over HashMap) with convenience methods.
    m.insert("button.color.bg".into(), Color::rgb24(0x40_40_48).into());
    m.insert("button.color.hot".into(), Color::rgb24(0x50_50_58).into());
    m.insert(
        "button.color.active".into(),
        Color::rgb24(0x60_60_68).into(),
    );
    m.insert("label.color.text".into(), Color::rgb24(0xf0_f0_ea).into());
    m.into()
}
