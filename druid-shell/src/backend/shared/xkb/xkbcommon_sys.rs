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

#![allow(unused, non_upper_case_globals, non_camel_case_types, non_snake_case)]
// unknown lints to make compile on older rust versions
#![cfg_attr(test, allow(unknown_lints, deref_nullptr))]
// generated code has some redundant static lifetimes, I don't think we can change that.
#![allow(clippy::redundant_static_lifetimes)]

use nix::libc::FILE;
include!(concat!(env!("OUT_DIR"), "/xkbcommon_sys.rs"));
