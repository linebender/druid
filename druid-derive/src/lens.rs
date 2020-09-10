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

use super::attr::{FieldKind, Fields, LensAttrs};
use proc_macro2::{Ident, Span};
use quote::quote;
use std::collections::HashSet;
use syn::{spanned::Spanned, Data, GenericParam, TypeParam};

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

    let fields = if let syn::Data::Struct(syn::DataStruct { fields, .. }) = &input.data {
        Fields::<LensAttrs>::parse_ast(fields)?
    } else {
        return Err(syn::Error::new(
            input.span(),
            "Lens implementations can only be derived from structs with named fields",
        ));
    };

    if fields.kind != FieldKind::Named {
        return Err(syn::Error::new(
            input.span(),
            "Lens implementations can only be derived from structs with named fields",
        ));
    }

    let twizzled_name = if is_camel_case(&ty.to_string()) {
        let temp_name = format!("{}_derived_lenses", to_snake_case(&ty.to_string()));
        proc_macro2::Ident::new(&temp_name, proc_macro2::Span::call_site())
    } else {
        return Err(syn::Error::new(
            ty.span(),
            "Lens implementations can only be derived from CamelCase types",
        ));
    };

    // Define lens types for each field
    let defs = fields.iter().filter(|f| !f.attrs.ignore).map(|f| {
        let field_name = &f.ident.unwrap_named();

        quote! {
            /// Lens for the field on #ty
            #[allow(non_camel_case_types)]
            #[derive(Debug, Copy, Clone)]
            pub struct #field_name;
        }
    });
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let used_params: HashSet<String> = input
        .generics
        .params
        .iter()
        .flat_map(|gp: &GenericParam| match gp {
            GenericParam::Type(TypeParam { ident, .. }) => Some(ident.to_string()),
            _ => None,
        })
        .collect();

    let gen_new_param = |name: &str| {
        let mut candidate: String = name.into();
        let mut count = 1usize;
        while used_params.contains(&candidate) {
            candidate = format!("{}_{}", name, count);
            count += 1;
        }
        Ident::new(&candidate, Span::call_site())
    };

    let func_ty_par = gen_new_param("F");
    let val_ty_par = gen_new_param("V");

    let impls = fields.iter().filter(|f| !f.attrs.ignore).map(|f| {
        let field_name = &f.ident.unwrap_named();
        let field_ty = &f.ty;

        quote! {
            impl #impl_generics druid::Lens<#ty#ty_generics, #field_ty> for #twizzled_name::#field_name #where_clause {
                fn with<#val_ty_par, #func_ty_par: FnOnce(&#field_ty) -> #val_ty_par>(&self, data: &#ty#ty_generics, f: #func_ty_par) -> #val_ty_par {
                    f(&data.#field_name)
                }

                fn with_mut<#val_ty_par, #func_ty_par: FnOnce(&mut #field_ty) -> #val_ty_par>(&self, data: &mut #ty#ty_generics, f: #func_ty_par) -> #val_ty_par {
                    f(&mut data.#field_name)
                }
            }
        }
    });

    let associated_items = fields.iter().filter(|f| !f.attrs.ignore).map(|f| {
        let field_name = &f.ident.unwrap_named();
        let lens_field_name = f.attrs.lens_name_override.as_ref().unwrap_or(&field_name);

        quote! {
            /// Lens for the corresponding field
            pub const #lens_field_name: #twizzled_name::#field_name = #twizzled_name::#field_name;
        }
    });

    let expanded = quote! {
        pub mod #twizzled_name {
            #(#defs)*
        }

        #(#impls)*

        #[allow(non_upper_case_globals)]
        impl #impl_generics #ty #ty_generics #where_clause {
            #(#associated_items)*
        }
    };

    Ok(expanded)
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
