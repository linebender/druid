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

//! parsing #[druid(attributes)]

use proc_macro2::{Ident, Literal, Span, TokenTree};
use syn::spanned::Spanned;
use syn::{Error, Meta, NestedMeta};

const BASE_ATTR_PATH: &str = "druid";
const IGNORE_ATTR_PATH: &str = "ignore";

/// The fields for a struct or an enum variant.
#[derive(Debug)]
pub struct Fields {
    pub kind: FieldKind,
    fields: Vec<Field>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub struct Field {
    pub ident: FieldIdent,
    /// `true` if this field should be ignored.
    pub ignore: bool,
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

        let mut ignore = false;

        for attr in field
            .attrs
            .iter()
            .filter(|attr| attr.path.is_ident(BASE_ATTR_PATH))
        {
            match attr.parse_meta()? {
                Meta::List(meta) => {
                    for nested in meta.nested.iter() {
                        match nested {
                            NestedMeta::Meta(Meta::Path(path))
                                if path.is_ident(IGNORE_ATTR_PATH) =>
                            {
                                if ignore {
                                    return Err(Error::new(nested.span(), "Duplicate attribute"));
                                }
                                ignore = true;
                            }
                            other => return Err(Error::new(other.span(), "Unknown attribute")),
                        }
                    }
                }
                other => {
                    return Err(Error::new(
                        other.span(),
                        "Expected attribute list (the form #[druid(one, two)])",
                    ))
                }
            }
        }
        Ok(Field { ident, ignore })
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
}
