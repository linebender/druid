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

//! The implementation for #[derive(Data)]

use crate::attr::{DataAttrs, Field, FieldKind, Fields};

use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Data, DataEnum, DataStruct};

pub(crate) fn derive_data_impl(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        Data::Struct(s) => derive_struct(&input, s),
        Data::Enum(e) => derive_enum(&input, e),
        Data::Union(u) => Err(syn::Error::new(
            u.union_token.span(),
            "Data implementations cannot be derived from unions",
        )),
    }
}

fn derive_struct(
    input: &syn::DeriveInput,
    s: &DataStruct,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ident = &input.ident;
    let impl_generics = generics_bounds(&input.generics);
    let (_, ty_generics, where_clause) = &input.generics.split_for_impl();

    let fields = Fields::<DataAttrs>::parse_ast(&s.fields)?;

    let diff = if fields.len() > 0 {
        let same_fns = fields
            .iter()
            .filter(|f| !f.attrs.ignore)
            .map(Field::same_fn_path_tokens);
        let fields = fields
            .iter()
            .filter(|f| !f.attrs.ignore)
            .map(Field::ident_tokens);
        quote!( #( #same_fns(&self.#fields, &other.#fields) )&&* )
    } else {
        quote!(true)
    };

    let res = quote! {
        impl<#impl_generics> ::druid::Data for #ident #ty_generics #where_clause {
            fn same(&self, other: &Self) -> bool {
                #diff
            }
        }
    };

    Ok(res)
}

fn ident_from_str(s: &str) -> proc_macro2::Ident {
    proc_macro2::Ident::new(s, proc_macro2::Span::call_site())
}

fn is_c_style_enum(s: &DataEnum) -> bool {
    s.variants.iter().all(|variant| match &variant.fields {
        syn::Fields::Named(fs) => fs.named.is_empty(),
        syn::Fields::Unnamed(fs) => fs.unnamed.is_empty(),
        syn::Fields::Unit => true,
    })
}

fn derive_enum(
    input: &syn::DeriveInput,
    s: &DataEnum,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ident = &input.ident;
    let impl_generics = generics_bounds(&input.generics);
    let (_, ty_generics, where_clause) = &input.generics.split_for_impl();

    if is_c_style_enum(&s) {
        let res = quote! {
            impl<#impl_generics> ::druid::Data for #ident #ty_generics #where_clause {
                fn same(&self, other: &Self) -> bool { self == other }
            }
        };
        return Ok(res);
    }

    let cases: Vec<proc_macro2::TokenStream> = s
        .variants
        .iter()
        .map(|variant| {
            let fields = Fields::<DataAttrs>::parse_ast(&variant.fields)?;
            let variant = &variant.ident;

            // the various inner `same()` calls, to the right of the match arm.
            let tests: Vec<_> = fields
                .iter()
                .filter(|field| !field.attrs.ignore)
                .map(|field| {
                    let same_fn = field.same_fn_path_tokens();
                    let var_left = ident_from_str(&format!("__self_{}", field.ident_string()));
                    let var_right = ident_from_str(&format!("__other_{}", field.ident_string()));
                    quote!( #same_fn(#var_left, #var_right) )
                })
                .collect();

            if let FieldKind::Named = fields.kind {
                let lefts: Vec<_> = fields
                    .iter()
                    .map(|field| {
                        let ident = field.ident_tokens();
                        let var = ident_from_str(&format!("__self_{}", field.ident_string()));
                        quote!( #ident: #var )
                    })
                    .collect();
                let rights: Vec<_> = fields
                    .iter()
                    .map(|field| {
                        let ident = field.ident_tokens();
                        let var = ident_from_str(&format!("__other_{}", field.ident_string()));
                        quote!( #ident: #var )
                    })
                    .collect();

                Ok(quote! {
                    (#ident :: #variant { #( #lefts ),* }, #ident :: #variant { #( #rights ),* }) => {
                        #( #tests )&&*
                    }
                })
            } else {
                let vars_left: Vec<_> = fields
                    .iter()
                    .map(|field| ident_from_str(&format!("__self_{}", field.ident_string())))
                    .collect();
                let vars_right: Vec<_> = fields
                    .iter()
                    .map(|field| ident_from_str(&format!("__other_{}", field.ident_string())))
                    .collect();

                if fields.iter().count() > 0 {
                    Ok(quote! {
                        ( #ident :: #variant( #(#vars_left),* ),  #ident :: #variant( #(#vars_right),* )) => {
                            #( #tests )&&*
                        }
                    })
                } else {
                    Ok(quote! {
                        ( #ident :: #variant ,  #ident :: #variant ) => { true }
                    })
                }
            }
        })
        .collect::<Result<Vec<proc_macro2::TokenStream>, syn::Error>>()?;

    let res = quote! {
        impl<#impl_generics> ::druid::Data for #ident #ty_generics #where_clause {
            fn same(&self, other: &Self) -> bool {
                match (self, other) {
                    #( #cases ),*
                    _ => false,
                }
            }
        }
    };

    Ok(res)
}

fn generics_bounds(generics: &syn::Generics) -> proc_macro2::TokenStream {
    let res = generics.params.iter().map(|gp| {
        use syn::GenericParam::*;
        match gp {
            Type(ty) => {
                let ident = &ty.ident;
                let bounds = &ty.bounds;
                if bounds.is_empty() {
                    quote_spanned!(ty.span()=> #ident : ::druid::Data)
                } else {
                    quote_spanned!(ty.span()=> #ident : #bounds + ::druid::Data)
                }
            }
            Lifetime(lf) => quote!(#lf),
            Const(cst) => quote!(#cst),
        }
    });

    quote!( #( #res, )* )
}
