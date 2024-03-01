// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! derive macros for Druid.

#![deny(clippy::trivially_copy_pass_by_ref)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/linebender/druid/screenshots/images/doc_logo.png"
)]

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
/// - `#[data(eq)]` is shorthand for `#[data(same_fn = "PartialEq::eq")]`
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
///     #[data(eq)]
///     // same as #[data(same_fn="PartialEq::eq")]
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
