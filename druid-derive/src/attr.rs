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

//! parsing #[druid(attributes)]

use proc_macro2::{Ident, Literal, Span, TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::{Error, ExprPath, Meta, NestedMeta};

use quote::{quote, quote_spanned};

//show error to tell users of old API that it doesn't work anymore
const BASE_DRUID_DEPRECATED_ATTR_PATH: &str = "druid";
const BASE_DATA_ATTR_PATH: &str = "data";
const BASE_LENS_ATTR_PATH: &str = "lens";
const IGNORE_ATTR_PATH: &str = "ignore";
const DATA_SAME_FN_ATTR_PATH: &str = "same_fn";
const LENS_NAME_OVERRIDE_ATTR_PATH: &str = "name";

/// The fields for a struct or an enum variant.
#[derive(Debug)]
pub struct Fields {
    pub kind: FieldKind,
    fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    Named,
    // this also covers Unit; we determine 'unit-ness' based on the number
    // of fields.
    Unnamed,
}

#[derive(Debug)]
pub enum FieldIdent {
    Named(String),
    Unnamed(usize),
}

impl FieldIdent {
    pub fn unwrap_named(&self) -> syn::Ident {
        if let FieldIdent::Named(s) = self {
            syn::Ident::new(&s, Span::call_site())
        } else {
            panic!("Unwrap named called on unnamed FieldIdent");
        }
    }
}

#[derive(Debug)]
pub struct Field {
    pub ident: FieldIdent,
    pub ty: syn::Type,

    /// `true` if this field should be ignored.
    pub ignore: bool,
    pub same_fn: Option<ExprPath>,
    pub lens_name_override: Option<Ident>,
    //TODO: more attrs here
}

impl Fields {
    pub fn parse_ast(fields: &syn::Fields) -> Result<Self, Error> {
        let kind = match fields {
            syn::Fields::Named(_) => FieldKind::Named,
            syn::Fields::Unnamed(_) | syn::Fields::Unit => FieldKind::Unnamed,
        };

        let fields = fields
            .iter()
            .enumerate()
            .map(|(i, field)| Field::parse_ast(field, i))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Fields { kind, fields })
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter()
    }
}

impl Field {
    pub fn parse_ast(field: &syn::Field, index: usize) -> Result<Self, Error> {
        let ident = match field.ident.as_ref() {
            Some(ident) => FieldIdent::Named(ident.to_string().trim_start_matches("r#").to_owned()),
            None => FieldIdent::Unnamed(index),
        };

        let ty = field.ty.clone();

        let mut ignore = false;
        let mut same_fn = None;
        let mut lens_name_override = None;

        for attr in field.attrs.iter() {
            if attr.path.is_ident(BASE_DRUID_DEPRECATED_ATTR_PATH) {
                panic!(
                    "The 'druid' attribute has been replaced with separate \
                    'lens' and 'data' attributes.",
                );
            } else if attr.path.is_ident(BASE_DATA_ATTR_PATH) {
                match attr.parse_meta()? {
                    Meta::List(meta) => {
                        for nested in meta.nested.iter() {
                            match nested {
                                NestedMeta::Meta(Meta::Path(path))
                                    if path.is_ident(IGNORE_ATTR_PATH) =>
                                {
                                    if ignore {
                                        return Err(Error::new(
                                            nested.span(),
                                            "Duplicate attribute",
                                        ));
                                    }
                                    ignore = true;
                                }
                                NestedMeta::Meta(Meta::NameValue(meta))
                                    if meta.path.is_ident(DATA_SAME_FN_ATTR_PATH) =>
                                {
                                    if same_fn.is_some() {
                                        return Err(Error::new(meta.span(), "Duplicate attribute"));
                                    }

                                    let path = parse_lit_into_expr_path(&meta.lit)?;
                                    same_fn = Some(path);
                                }
                                other => return Err(Error::new(other.span(), "Unknown attribute")),
                            }
                        }
                    }
                    other => {
                        return Err(Error::new(
                            other.span(),
                            "Expected attribute list (the form #[data(one, two)])",
                        ));
                    }
                }
            } else if attr.path.is_ident(BASE_LENS_ATTR_PATH) {
                match attr.parse_meta()? {
                    Meta::List(meta) => {
                        for nested in meta.nested.iter() {
                            match nested {
                                NestedMeta::Meta(Meta::NameValue(meta))
                                    if meta.path.is_ident(LENS_NAME_OVERRIDE_ATTR_PATH) =>
                                {
                                    if lens_name_override.is_some() {
                                        return Err(Error::new(meta.span(), "Duplicate attribute"));
                                    }

                                    let ident = parse_lit_into_ident(&meta.lit)?;
                                    lens_name_override = Some(ident);
                                }
                                other => return Err(Error::new(other.span(), "Unknown attribute")),
                            }
                        }
                    }
                    other => {
                        return Err(Error::new(
                            other.span(),
                            "Expected attribute list (the form #[data(one, two)])",
                        ));
                    }
                }
            }
        }
        Ok(Field {
            ident,
            ty,
            ignore,
            same_fn,
            lens_name_override,
        })
    }

    pub fn ident_tokens(&self) -> TokenTree {
        match self.ident {
            FieldIdent::Named(ref s) => Ident::new(&s, Span::call_site()).into(),
            FieldIdent::Unnamed(num) => Literal::usize_unsuffixed(num).into(),
        }
    }

    pub fn ident_string(&self) -> String {
        match self.ident {
            FieldIdent::Named(ref s) => s.clone(),
            FieldIdent::Unnamed(num) => num.to_string(),
        }
    }

    /// The tokens to be used as the function for 'same'.
    pub fn same_fn_path_tokens(&self) -> TokenStream {
        match self.same_fn {
            Some(ref f) => quote!(#f),
            None => {
                let span = Span::call_site();
                quote_spanned!(span=> druid::Data::same)
            }
        }
    }
}

fn parse_lit_into_expr_path(lit: &syn::Lit) -> Result<ExprPath, Error> {
    let string = if let syn::Lit::Str(lit) = lit {
        lit
    } else {
        return Err(Error::new(
            lit.span(),
            "expected str, found... something else",
        ));
    };

    let tokens = syn::parse_str(&string.value())?;
    syn::parse2(tokens)
}

fn parse_lit_into_ident(lit: &syn::Lit) -> Result<Ident, Error> {
    let ident = if let syn::Lit::Str(lit) = lit {
        Ident::new(&lit.value(), lit.span())
    } else {
        return Err(Error::new(
            lit.span(),
            "expected str, found... something else",
        ));
    };

    Ok(ident)
}
