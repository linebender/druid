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

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{parse_macro_input, spanned::Spanned, Data, DataEnum, DataStruct, Fields};

#[proc_macro_derive(Data, attributes(druid))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    derive_inner(input)
        .unwrap_or_else(|err| err.to_compile_error().into())
        .into()
}

fn derive_inner(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        Data::Struct(s) => derive_struct(&input, s),
        Data::Enum(e) => derive_enum(&input, e),
        Data::Union(u) => Err(syn::Error::new(
            u.union_token.span(),
            "Data implementations cannot be derived from unions",
        )),
    }
}

fn number_to_tokenstream(i: usize) -> proc_macro2::TokenStream {
    let lit = proc_macro2::Literal::usize_unsuffixed(i);
    let lit: proc_macro2::TokenTree = lit.into();
    lit.into()
}

enum EnumKind {
    Named,
    Unnamed,
}

fn extract_fields(
    fs: &syn::Fields,
) -> Result<(EnumKind, Vec<proc_macro2::TokenStream>), syn::Error> {
    match fs {
        Fields::Named(fs) => {
            let idents = fs
                .named
                .iter()
                .filter_map(|field| match should_ignore_field(field) {
                    Ok(true) => None,
                    Ok(false) => {
                        let ident = field.ident.as_ref().expect("expected named field");
                        Some(Ok(quote_spanned!(ident.span()=> #ident)))
                    }
                    Err(e) => Some(Err(e)),
                })
                .collect::<Result<_, _>>()?;
            Ok((EnumKind::Named, idents))
        }
        Fields::Unnamed(fs) => {
            let idents = fs
                .unnamed
                .iter()
                .enumerate()
                .filter_map(|(i, field)| match should_ignore_field(field) {
                    Ok(true) => None,
                    Ok(false) => Some(Ok(number_to_tokenstream(i))),
                    Err(e) => Some(Err(e)),
                })
                .collect::<Result<_, _>>()?;
            Ok((EnumKind::Unnamed, idents))
        }
        Fields::Unit => Ok((EnumKind::Unnamed, Vec::default())),
    }
}

/// Looks for an attribute of the format `druid(ignore)`.
///
/// Returns an error if there are confusing or unexpected attributes.
//TODO: if we ever get additional attributes we need to be more systematic;
//we should have an Attrs struct that we parse for all the attributes we know,
//and then pass that along.
fn should_ignore_field(field: &syn::Field) -> Result<bool, syn::Error> {
    for attr in field.attrs.iter() {
        if !attr.path.is_ident("druid") {
            continue;
        }
        match attr.parse_meta()? {
            syn::Meta::List(meta) => {
                for nested in meta.nested.iter() {
                    if let syn::NestedMeta::Meta(syn::Meta::Path(path)) = nested {
                        if path.is_ident("ignore") {
                            return Ok(true);
                        } else {
                            return Err(syn::Error::new(path.span(), "Unknown attribute"));
                        }
                    } else {
                        return Err(syn::Error::new(nested.span(), "Unknown attribute"));
                    }
                }
            }
            other => return Err(syn::Error::new(other.span(), "Unknown attribute")),
        }
    }
    Ok(false)
}

fn derive_struct(
    input: &syn::DeriveInput,
    s: &DataStruct,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let generics_bounds = generics_bounds(&input.generics);
    let generics = &input.generics;

    let ty = &input.ident;
    let fields = extract_fields(&s.fields)?.1;

    let res = quote! {
        impl<#generics_bounds> druid::Data for #ty #generics {
            fn same(&self, other: &Self) -> bool {
                #( self.#fields.same(&other.#fields) )&&*
            }
        }
    };

    Ok(res.into())
}

fn ident_from_str(s: &str) -> proc_macro2::Ident {
    proc_macro2::Ident::new(s, proc_macro2::Span::call_site())
}

fn is_c_style_enum(s: &DataEnum) -> bool {
    s.variants.iter().all(|variant| {
        use Fields::*;
        match &variant.fields {
            Named(fs) => fs.named.is_empty(),
            Unnamed(fs) => fs.unnamed.is_empty(),
            Unit => true,
        }
    })
}

fn derive_enum(
    input: &syn::DeriveInput,
    s: &DataEnum,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ty = &input.ident;

    if is_c_style_enum(&s) {
        let generics_bounds = generics_bounds(&input.generics);
        let generics = &input.generics;

        let res = quote! {
            impl<#generics_bounds> ::druid::Data for #ty #generics {
                fn same(&self, other: &Self) -> bool { self == other }
            }
        };
        return Ok(res);
    }

    let cases: Vec<proc_macro2::TokenStream> = s
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            let (kind, idents) = extract_fields(&variant.fields)?;

            let tests: Vec<_> = idents
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let var_left = ident_from_str(&format!("left{}", i));
                    let var_right = ident_from_str(&format!("right{}", i));
                    quote!( #var_left.same(#var_right) )
                })
                .collect();

            if let EnumKind::Named = kind {
                let lefts: Vec<_> = idents
                    .iter()
                    .enumerate()
                    .map(|(i, ident)| {
                        let var = ident_from_str(&format!("left{}", i));
                        quote!( #ident: #var )
                    })
                    .collect();
                let rights: Vec<_> = idents
                    .iter()
                    .enumerate()
                    .map(|(i, ident)| {
                        let var = ident_from_str(&format!("right{}", i));
                        quote!( #ident: #var )
                    })
                    .collect();

                Ok(quote! {
                    (#ty :: #ident { #( #lefts ),* }, #ty :: #ident { #( #rights ),* }) => {
                        #( #tests )&&*
                    }
                })
            } else {
                let vars_left: Vec<_> = idents
                    .iter()
                    .enumerate()
                    .map(|(i, _)| ident_from_str(&format!("left{}", i)))
                    .collect();
                let vars_right: Vec<_> = idents
                    .iter()
                    .enumerate()
                    .map(|(i, _)| ident_from_str(&format!("right{}", i)))
                    .collect();

                if idents.len() > 0 {
                    Ok(quote! {
                        ( #ty :: #ident( #(#vars_left),* ),  #ty :: #ident( #(#vars_right),* )) => {
                            #( #tests )&&*
                        }
                    })
                } else {
                    Ok(quote! {
                       ( #ty :: #ident ,  #ty :: #ident ) => { true }
                    })
                }
            }
        })
        .collect::<Result<Vec<proc_macro2::TokenStream>, syn::Error>>()?;

    let generics_bounds = generics_bounds(&input.generics);
    let generics = &input.generics;

    let res = quote! {
        impl<#generics_bounds> ::druid::Data for #ty #generics {
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
            Type(ty) => quote_spanned!(ty.span()=> #ty : ::druid::Data),
            Lifetime(lf) => quote!(#lf),
            Const(cst) => quote!(#cst),
        }
    });

    quote!( #( #res, )* )
}
