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

use quote::quote;
use syn::{spanned::Spanned, Data};

pub(crate) fn derive_lens_impl(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        Data::Struct(_) => derive_struct(&input),
        Data::Enum(e) => Err(syn::Error::new(
            e.enum_token.span(),
            "Lens implementations cannot be derived from enums",
        )),
        Data::Union(u) => Err(syn::Error::new(
            u.union_token.span(),
            "Lens implementations cannot be derived from unions",
        )),
    }
}

fn derive_struct(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ty = &input.ident;

    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = input.data
    {
        named
    } else {
        return Err(syn::Error::new(
            input.span(),
            "Lens implementations can only be derived from structs with named fields",
        ));
    };

    let twizzled_name = if is_camel_case(&ty.to_string()) {
        let temp_name = to_snake_case(&mut ty.to_string());
        proc_macro2::Ident::new(&temp_name, proc_macro2::Span::call_site())
    } else {
        return Err(syn::Error::new(
            ty.span(),
            "Lens implementations can only be derived from CamelCase types",
        ));
    };

    // Declare a struct for each field
    let structs = fields.iter().map(|f| {
        let field_name = &f.ident;

        quote! {
            #[allow(non_camel_case_types)]
            pub struct #field_name;
        }
    });

    // Impl Lens for each field
    let impls = fields.iter().map(|f| {
        let field_name = &f.ident;
        let field_ty = &f.ty;

        quote! {

            impl Lens<#ty, #field_ty> for #field_name {
                fn get<'a>(&self, data: &'a #ty) -> &'a #field_ty {
                    &data.#field_name
                }

                fn with_mut<V, F: FnOnce(&mut #field_ty) -> V>(&self, data: &mut #ty, f: F) -> V {
                    f(&mut data.#field_name)
                }
            }
        }
    });

    let expanded = quote! {
        pub mod lenses {
            pub mod #twizzled_name {
                use super::super::*;

                use druid::Lens;
                #(#structs)*
                #(#impls)*
            }

        }
    };

    Ok(expanded.into())
}

//I stole these from rustc!
fn char_has_case(c: char) -> bool {
    c.is_lowercase() || c.is_uppercase()
}

fn is_camel_case(name: &str) -> bool {
    let name = name.trim_matches('_');
    if name.is_empty() {
        return true;
    }

    // start with a non-lowercase letter rather than non-uppercase
    // ones (some scripts don't have a concept of upper/lowercase)
    !name.chars().next().unwrap().is_lowercase()
        && !name.contains("__")
        && !name.chars().collect::<Vec<_>>().windows(2).any(|pair| {
            // contains a capitalisable character followed by, or preceded by, an underscore
            char_has_case(pair[0]) && pair[1] == '_' || char_has_case(pair[1]) && pair[0] == '_'
        })
}

fn to_snake_case(mut str: &str) -> String {
    let mut words = vec![];
    // Preserve leading underscores
    str = str.trim_start_matches(|c: char| {
        if c == '_' {
            words.push(String::new());
            true
        } else {
            false
        }
    });
    for s in str.split('_') {
        let mut last_upper = false;
        let mut buf = String::new();
        if s.is_empty() {
            continue;
        }
        for ch in s.chars() {
            if !buf.is_empty() && buf != "'" && ch.is_uppercase() && !last_upper {
                words.push(buf);
                buf = String::new();
            }
            last_upper = ch.is_uppercase();
            buf.extend(ch.to_lowercase());
        }
        words.push(buf);
    }
    words.join("_")
}
