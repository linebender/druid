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

//! derive macros for druid.

extern crate proc_macro;

mod attr;
mod data;
mod lens;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Generates implementations of the `Data` trait.
#[proc_macro_derive(Data, attributes(data))]
pub fn derive_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    data::derive_data_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Generates lenses to access the fields of a struct
///
/// An associated constant is defined on the struct for each field,
/// having the same name as the field.
#[proc_macro_derive(Lens, attributes(lens))]
pub fn derive_lens(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    lens::derive_lens_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
