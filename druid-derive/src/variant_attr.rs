use super::field_attr::{Field, Fields};
use proc_macro2::Span;
use syn::Error;

#[derive(Debug)]
pub struct Variants {
    variants: Vec<Variant>,
}

#[derive(Debug)]
pub struct Variant {
    pub ident: StringIdent,
    pub field: Field,
}

#[derive(Debug)]
pub struct StringIdent(pub String);

impl StringIdent {
    pub fn named(&self) -> syn::Ident {
        syn::Ident::new(self.0.as_ref(), Span::call_site())
    }
}

impl Variants {
    pub fn parse_ast(
        variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
    ) -> Result<Self, Error> {
        let variants = variants
            .iter()
            .enumerate()
            .map(|(i, variant)| Variant::parse_ast(variant, i))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Variants { variants })
    }

    pub fn _len(&self) -> usize {
        self.variants.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Variant> {
        self.variants.iter()
    }
}

impl Variant {
    pub fn parse_ast(variant: &syn::Variant, _index: usize) -> Result<Self, Error> {
        let ident = variant
            .ident
            .to_string()
            .trim_start_matches("r#")
            .to_owned();
        let fields = Fields::parse_ast(&variant.fields)?;
        let field = if let Some(field) = fields.iter().next() {
            if fields.iter().count() > 1 {
                return Err(Error::new(
                    variant.ident.span(),
                    "Expecting no more than one field for the variant",
                ));
            } else {
                field
            }
        } else {
            return Err(Error::new(
                variant.ident.span(),
                "Expecting one field for the variant",
            ));
        };

        Ok(Variant {
            ident: StringIdent(ident),
            field: field.clone(),
        })
    }
}
