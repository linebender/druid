// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

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
const DATA_EQ_ATTR_PATH: &str = "eq";
const LENS_NAME_OVERRIDE_ATTR_PATH: &str = "name";

/// The fields for a struct or an enum variant.
#[derive(Debug)]
pub struct Fields<Attrs> {
    pub kind: FieldKind,
    fields: Vec<Field<Attrs>>,
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
            syn::Ident::new(s, Span::call_site())
        } else {
            panic!("Unwrap named called on unnamed FieldIdent");
        }
    }
}

#[derive(Debug)]
pub struct Field<Attrs> {
    pub ident: FieldIdent,
    pub ty: syn::Type,

    pub attrs: Attrs,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DataAttr {
    Empty,
    Ignore,
    SameFn(ExprPath),
    Eq,
}

#[derive(Debug)]
pub struct LensAttrs {
    /// `true` if this field should be ignored.
    pub ignore: bool,
    pub lens_name_override: Option<Ident>,
}

impl Fields<DataAttr> {
    pub fn parse_ast(fields: &syn::Fields) -> Result<Self, Error> {
        let kind = match fields {
            syn::Fields::Named(_) => FieldKind::Named,
            syn::Fields::Unnamed(_) | syn::Fields::Unit => FieldKind::Unnamed,
        };

        let fields = fields
            .iter()
            .enumerate()
            .map(|(i, field)| Field::<DataAttr>::parse_ast(field, i))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Fields { kind, fields })
    }
}

impl Fields<LensAttrs> {
    pub fn parse_ast(fields: &syn::Fields) -> Result<Self, Error> {
        let kind = match fields {
            syn::Fields::Named(_) => FieldKind::Named,
            syn::Fields::Unnamed(_) | syn::Fields::Unit => FieldKind::Unnamed,
        };

        let fields = fields
            .iter()
            .enumerate()
            .map(|(i, field)| Field::<LensAttrs>::parse_ast(field, i))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Fields { kind, fields })
    }
}

impl<Attrs> Fields<Attrs> {
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Field<Attrs>> {
        self.fields.iter()
    }
}

impl Field<DataAttr> {
    pub fn parse_ast(field: &syn::Field, index: usize) -> Result<Self, Error> {
        let ident = match field.ident.as_ref() {
            Some(ident) => FieldIdent::Named(ident.to_string().trim_start_matches("r#").to_owned()),
            None => FieldIdent::Unnamed(index),
        };

        let ty = field.ty.clone();

        let mut data_attr = DataAttr::Empty;
        for attr in field.attrs.iter() {
            if attr.path.is_ident(BASE_DRUID_DEPRECATED_ATTR_PATH) {
                panic!(
                    "The 'druid' attribute has been replaced with separate \
                    'lens' and 'data' attributes.",
                );
            } else if attr.path.is_ident(BASE_DATA_ATTR_PATH) {
                match attr.parse_meta()? {
                    Meta::List(meta) => {
                        assert!(
                            meta.nested.len() <= 1,
                            "only single data attribute is allowed"
                        );
                        if let Some(nested) = meta.nested.first() {
                            match nested {
                                NestedMeta::Meta(Meta::Path(path))
                                    if path.is_ident(IGNORE_ATTR_PATH) =>
                                {
                                    data_attr = DataAttr::Ignore;
                                }
                                NestedMeta::Meta(Meta::NameValue(meta))
                                    if meta.path.is_ident(DATA_SAME_FN_ATTR_PATH) =>
                                {
                                    let path = parse_lit_into_expr_path(&meta.lit)?;
                                    data_attr = DataAttr::SameFn(path);
                                }
                                NestedMeta::Meta(Meta::Path(path))
                                    if path.is_ident(DATA_EQ_ATTR_PATH) =>
                                {
                                    data_attr = DataAttr::Eq;
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
            attrs: data_attr,
        })
    }

    /// The tokens to be used as the function for 'same'.
    pub fn same_fn_path_tokens(&self) -> TokenStream {
        match &self.attrs {
            DataAttr::SameFn(f) => quote!(#f),
            DataAttr::Eq => quote!(::core::cmp::PartialEq::eq),
            // this should not be called for DataAttr::Ignore
            DataAttr::Ignore => quote!(compiler_error!),
            DataAttr::Empty => {
                let span = Span::call_site();
                quote_spanned!(span=> druid::Data::same)
            }
        }
    }
}

impl Field<LensAttrs> {
    pub fn parse_ast(field: &syn::Field, index: usize) -> Result<Self, Error> {
        let ident = match field.ident.as_ref() {
            Some(ident) => FieldIdent::Named(ident.to_string().trim_start_matches("r#").to_owned()),
            None => FieldIdent::Unnamed(index),
        };

        let ty = field.ty.clone();

        let mut ignore = false;
        let mut lens_name_override = None;

        for attr in field.attrs.iter() {
            if attr.path.is_ident(BASE_DRUID_DEPRECATED_ATTR_PATH) {
                panic!(
                    "The 'druid' attribute has been replaced with separate \
                    'lens' and 'data' attributes.",
                );
            } else if attr.path.is_ident(BASE_LENS_ATTR_PATH) {
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
                            "Expected attribute list (the form #[lens(one, two)])",
                        ));
                    }
                }
            }
        }
        Ok(Field {
            ident,
            ty,
            attrs: LensAttrs {
                ignore,
                lens_name_override,
            },
        })
    }
}

impl<Attrs> Field<Attrs> {
    pub fn ident_tokens(&self) -> TokenTree {
        match self.ident {
            FieldIdent::Named(ref s) => Ident::new(s, Span::call_site()).into(),
            FieldIdent::Unnamed(num) => Literal::usize_unsuffixed(num).into(),
        }
    }

    pub fn ident_string(&self) -> String {
        match self.ident {
            FieldIdent::Named(ref s) => s.clone(),
            FieldIdent::Unnamed(num) => num.to_string(),
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
