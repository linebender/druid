// Copyright 2020 The xi-editor Authors.
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

use wasm_bindgen::prelude::*;

mod examples;

macro_rules! impl_example {
    ($wasm_fn:ident, $example:ident) => {
        #[wasm_bindgen]
        pub fn $wasm_fn() {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            examples::$example::main();
        }
    };
    ($fn:ident) => {
        impl_example!($fn, $fn);
    };
}

impl_example!(anim);
impl_example!(calc);
impl_example!(custom_widget);
impl_example!(either);
//impl_example!(ext_event); // No thread support on wasm
impl_example!(flex);
impl_example!(game_of_life);
impl_example!(hello);
impl_example!(identity);
impl_example!(image);
impl_example!(layout);
impl_example!(lens);
impl_example!(list);
impl_example!(multiwin);
impl_example!(panels);
impl_example!(parse);
impl_example!(scroll_colors);
impl_example!(scroll);
impl_example!(split_demo);
impl_example!(styled_text);
//impl_example!(svg); // usvg doesn't compile on usvg at the time of this writing
impl_example!(switch_demo, switch);
impl_example!(timer);
impl_example!(view_switcher);
