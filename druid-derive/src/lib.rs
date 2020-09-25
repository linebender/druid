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

//! derive macros for druid.

#![deny(clippy::trivially_copy_pass_by_ref)]

extern crate proc_macro;

mod attr;
mod data;
mod lens;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Generates implementations of the `Data` trait.
///
/// This macro supports a `data` field attribute with the following arguments:
///
/// - `#[data(ignore)]` makes the generated `Data::same` function skip comparing this field.
/// - `#[data(same_fn="foo")]` uses the function `foo` for comparing this field. `foo` should
///    be the name of a function with signature `fn(&T, &T) -> bool`, where `T` is the type of
///    the field.
///
/// # Example
///
/// ```rust
/// use druid_derive::Data;
///
/// #[derive(Clone, Data)]
/// struct State {
///     number: f64,
///     // `Vec` doesn't implement `Data`, so we need to either ignore it or supply a `same_fn`.
///     #[data(same_fn="PartialEq::eq")]
///     indices: Vec<usize>,
///     // This is just some sort of cache; it isn't important for sameness comparison.
///     #[data(ignore)]
///     cached_indices: Vec<usize>,
/// }
/// ```
#[proc_macro_derive(Data, attributes(data))]
pub fn derive_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    data::derive_data_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Generates lenses to access the fields of a struct.
///
/// An associated constant is defined on the struct for each field,
/// having the same name as the field.
///
/// This macro supports a `lens` field attribute with the following arguments:
///
/// - `#[lens(ignore)]` skips creating a lens for one field.
/// - `#[lens(name="foo")]` gives the lens the specified name (instead of the default, which is to
///    create a lens with the same name as the field).
///
/// # Example
///
/// ```rust
/// use druid_derive::Lens;
///
/// #[derive(Lens)]
/// struct State {
///     // The Lens derive will create a `State::text` constant implementing
///     // `druid::Lens<State, String>`
///     text: String,
///     // The Lens derive will create a `State::lens_number` constant implementing
///     // `druid::Lens<State, f64>`
///     #[lens(name = "lens_number")]
///     number: f64,
///     // The Lens derive won't create anything for this field.
///     #[lens(ignore)]
///     blah: f64,
/// }
/// ```
#[proc_macro_derive(Lens, attributes(lens))]
pub fn derive_lens(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    lens::derive_lens_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
