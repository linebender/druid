use super::field_attr::{self, Fields, LensAttrs};
use proc_macro2::Span;
use syn::{Error, Ident, Meta, NestedMeta};

pub const BASE_PRISM_ATTR_PATH: &str = "prism";
pub const IGNORE_VARIANT_ATTR_PATH: &str = "ignore_variant";
pub const PRISM_NAME_OVERRIDE_ATTR_PATH: &str = "name";

#[derive(Debug)]
pub struct Variants<Attrs, FieldAttrs> {
    variants: Vec<Variant<Attrs, FieldAttrs>>,
}

#[derive(Debug)]
pub struct Variant<Attrs, FieldAttrs> {
    pub ident: StringIdent,
    pub fields: Fields<FieldAttrs>,

    pub attrs: Attrs,
}

#[derive(Debug)]
pub struct DataAttrs {
    /// `true` if this variant should be ignored.
    pub ignore_variant: bool,
    pub same_fn: Option<syn::ExprPath>,
}

#[derive(Debug)]
pub struct PrismAttrs {
    /// `true` if this variant should be ignored.
    pub ignore_variant: bool,
    pub prism_name_override: Option<Ident>,
}

#[derive(Debug)]
pub struct StringIdent(pub String);

impl StringIdent {
    pub fn named(&self) -> syn::Ident {
        syn::Ident::new(self.0.as_ref(), Span::call_site())
    }
}

impl Variants<DataAttrs, field_attr::DataAttrs> {
    pub fn parse_ast(
        variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
    ) -> Result<Self, Error> {
        let variants = variants
            .iter()
            .enumerate()
            .map(|(i, variant)| Variant::<DataAttrs, field_attr::DataAttrs>::parse_ast(variant, i))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Variants { variants })
    }
}

impl Variants<PrismAttrs, LensAttrs> {
    pub fn parse_ast(
        variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
    ) -> Result<Self, Error> {
        let variants = variants
            .iter()
            .enumerate()
            .map(|(i, variant)| Variant::<PrismAttrs, LensAttrs>::parse_ast(variant, i))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Variants { variants })
    }
}

impl<Attrs, FieldAttrs> Variants<Attrs, FieldAttrs> {
    pub fn _len(&self) -> usize {
        self.variants.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Variant<Attrs, FieldAttrs>> {
        self.variants.iter()
    }
}

impl Variant<DataAttrs, field_attr::DataAttrs> {
    pub fn parse_ast(variant: &syn::Variant, _index: usize) -> Result<Self, Error> {
        let ident = variant
            .ident
            .to_string()
            .trim_start_matches("r#")
            .to_owned();
        let fields = Fields::<field_attr::DataAttrs>::parse_ast(&variant.fields)?;

        let mut ignore_variant = false;
        let mut same_fn = None;

        for attr in variant.attrs.iter() {
            if attr
                .path
                .is_ident(field_attr::BASE_DRUID_DEPRECATED_ATTR_PATH)
            {
                panic!(
                    "The 'druid' attribute has been replaced with separate \
                    'lens', 'prism' and 'data' attributes.",
                );
            } else if attr.path.is_ident(field_attr::BASE_DATA_ATTR_PATH) {
                use syn::spanned::Spanned;
                match attr.parse_meta()? {
                    Meta::List(meta) => {
                        for nested in meta.nested.iter() {
                            match nested {
                                NestedMeta::Meta(Meta::Path(path))
                                    if path.is_ident(IGNORE_VARIANT_ATTR_PATH) =>
                                {
                                    if ignore_variant {
                                        return Err(Error::new(
                                            nested.span(),
                                            "Duplicate attribute",
                                        ));
                                    }
                                    ignore_variant = true;
                                }
                                NestedMeta::Meta(Meta::NameValue(meta))
                                    if meta.path.is_ident(field_attr::DATA_SAME_FN_ATTR_PATH) =>
                                {
                                    if same_fn.is_some() {
                                        return Err(Error::new(meta.span(), "Duplicate attribute"));
                                    }

                                    let path = field_attr::parse_lit_into_expr_path(&meta.lit)?;
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
            }
        }

        Ok(Variant {
            ident: StringIdent(ident),
            fields,
            attrs: DataAttrs {
                ignore_variant,
                same_fn,
            },
        })
    }
}

impl Variant<PrismAttrs, LensAttrs> {
    pub fn parse_ast(variant: &syn::Variant, _index: usize) -> Result<Self, Error> {
        let ident = variant
            .ident
            .to_string()
            .trim_start_matches("r#")
            .to_owned();
        let fields = Fields::<LensAttrs>::parse_ast(&variant.fields)?;

        let mut ignore_variant = false;
        let mut prism_name_override = None;

        for attr in variant.attrs.iter() {
            if attr
                .path
                .is_ident(field_attr::BASE_DRUID_DEPRECATED_ATTR_PATH)
            {
                panic!(
                    "The 'druid' attribute has been replaced with separate \
                    'lens', 'prism' and 'data' attributes.",
                );
            } else if attr.path.is_ident(BASE_PRISM_ATTR_PATH) {
                use syn::spanned::Spanned;
                match attr.parse_meta()? {
                    Meta::List(meta) => {
                        for nested in meta.nested.iter() {
                            match nested {
                                NestedMeta::Meta(Meta::Path(path))
                                    if path.is_ident(IGNORE_VARIANT_ATTR_PATH) =>
                                {
                                    if ignore_variant {
                                        return Err(Error::new(
                                            nested.span(),
                                            "Duplicate attribute",
                                        ));
                                    }
                                    ignore_variant = true;
                                }
                                NestedMeta::Meta(Meta::NameValue(meta))
                                    if meta.path.is_ident(PRISM_NAME_OVERRIDE_ATTR_PATH) =>
                                {
                                    if prism_name_override.is_some() {
                                        return Err(Error::new(meta.span(), "Duplicate attribute"));
                                    }

                                    let ident = field_attr::parse_lit_into_ident(&meta.lit)?;
                                    prism_name_override = Some(ident);
                                }
                                other => return Err(Error::new(other.span(), "Unknown attribute")),
                            }
                        }
                    }
                    other => {
                        return Err(Error::new(
                            other.span(),
                            "Expected attribute list (the form #[prism(one, two)])",
                        ));
                    }
                }
            }
        }

        Ok(Variant {
            ident: StringIdent(ident),
            fields,
            attrs: PrismAttrs {
                ignore_variant,
                prism_name_override,
            },
        })
    }
}
