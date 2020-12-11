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

//! Build script to modify the build environment

use std::env;

fn main() {
    let env = env::var("TARGET").unwrap();
    if env.contains("windows") {
        // Since https://github.com/rust-lang/rust/pull/56568, shell32 is not included in all
        // Windows binaries by default.
        println!("cargo:rustc-link-lib=shell32");
    }
}
