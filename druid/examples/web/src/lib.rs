// Copyright 2020 The Druid Authors.
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

// This line includes an automatically generated (in build.rs) examples module.
// This particular mechanism is chosen to avoid any kinds of modifications to committed files at
// build time, keeping the source tree clean from build artifacts.
include!("examples.in");

// This macro constructs a `wasm_bindgen` entry point to the given example from the examples
// directory.
//
// There are three ways to call this macro:
//
// 1. impl_example!(<example_name>);
//    Creates the binding for an example whose main fn returns nothing (or unit).
//
// 2. impl_example!(<example_name>.unwrap());
//    Creates the binding for an example whose main fn returns a Result which is immediately
//    unwrapped.
//
// 3. impl_example!(<wasm_fn_name>, <path_to_example_main_fn>());
//    Creates a wasm binding named <wasm_fn_name>, which calls into the Rust example fn given by
//    <path_to_example_main_fn>. This can be used to make a different wasm binding name than the
//    name of the original example itself (e.g. it was used for the `switch` example to avoid name
//    collisions with the JavaScript `switch` statement).
macro_rules! impl_example {
    ($wasm_fn:ident, $expr:expr) => {
        #[wasm_bindgen]
        pub fn $wasm_fn() {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            $expr;
        }
    };
    ($fn:ident) => {
        impl_example!($fn, examples::$fn::main());
    };
    ($fn:ident.unwrap()) => {
        impl_example!($fn, examples::$fn::main().unwrap());
    };
}

// Below is a list of examples that can be built for the web.
// Please add the examples that cannot be built to the EXCEPTIONS list in build.rs.
impl_example!(anim);
impl_example!(calc);
impl_example!(custom_widget);
impl_example!(either);
impl_example!(flex.unwrap());
impl_example!(game_of_life);
impl_example!(hello);
impl_example!(identity);
impl_example!(image);
impl_example!(invalidation);
impl_example!(layout);
impl_example!(lens);
impl_example!(list);
impl_example!(multiwin);
impl_example!(open_save);
impl_example!(panels.unwrap());
impl_example!(parse);
impl_example!(scroll_colors);
impl_example!(scroll);
impl_example!(split_demo);
impl_example!(styled_text.unwrap());
impl_example!(switches);
impl_example!(timer);
impl_example!(view_switcher);
impl_example!(widget_gallery);
