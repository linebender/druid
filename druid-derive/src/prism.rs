// TODO: test the generics part

use super::field_attr::{FieldIdent, LensAttrs};
use super::lens::{is_camel_case, to_snake_case};
use super::variant_attr::{PrismAttrs, StringIdent, Variants};
use proc_macro2::{Ident, Span};
use quote::quote;
use std::collections::HashSet;
use syn::{spanned::Spanned, Data, Error, GenericParam, TypeParam};

pub(crate) fn derive_prism_impl(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        Data::Struct(s) => Err(syn::Error::new(
            s.struct_token.span(),
            "Prism implementations cannot be derived from structs",
        )),
        Data::Enum(_) => derive_enum(&input),
        Data::Union(u) => Err(syn::Error::new(
            u.union_token.span(),
            "Prism implementations cannot be derived from unions",
        )),
    }
}

fn derive_enum(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ty = &input.ident;

    let variants = if let syn::Data::Enum(syn::DataEnum { variants, .. }) = &input.data {
        Variants::<PrismAttrs, LensAttrs>::parse_ast(variants)?
    } else {
        return Err(syn::Error::new(
            input.span(),
            "Prism implementations can only be derived from .. (TODO)",
        ));
    };

    let twizzled_name = if is_camel_case(&ty.to_string()) {
        let temp_name = format!("{}_derived_prism", to_snake_case(&ty.to_string()));
        proc_macro2::Ident::new(&temp_name, proc_macro2::Span::call_site())
    } else {
        return Err(syn::Error::new(
            ty.span(),
            "Prism implementations can only be derived from CamelCase types",
        ));
    };

    // Define prism types for each variant
    let defs = variants.iter().filter(|v| !v.attrs.ignore).map(|v| {
        let variant_name = &v.ident.named();

        quote! {
            /// Prism for the variant on (the enum)
            #[allow(non_camel_case_types)]
            #[derive(Debug, Copy, Clone, PartialEq)]
            pub struct #variant_name;
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

    let impls: Vec<_> = variants
        .iter()
        .filter(|v| !v.attrs.ignore)
        .map(|v| {
            let variant_name = &v.ident.named();
            let field = if let Some(field) = v.fields.iter().next() {
                if v.fields.iter().count() > 1 {
                    return Err(Error::new(
                        variant_name.span(),
                        "Expecting no more than one field for the variant",
                    ));
                } else {
                    field
                }
            } else {
                return Err(Error::new(
                    variant_name.span(),
                    "Expecting one field for the variant",
                ));
            };
            let field_ty = &field.ty;

            let with_expr = match &field.ident {
                FieldIdent::Named(name) => {
                    let field_name = StringIdent(name.into()).named();
                    quote!(
                        if let #ty::#variant_name { #field_name } = data {
                            Some(f(#field_name))
                        } else {
                            None
                        }
                    )
                }
                FieldIdent::Unnamed(0) => {
                    let field_name = StringIdent("inner".into()).named();
                    quote!(
                        if let #ty::#variant_name (#field_name) = data {
                            Some(f(#field_name))
                        } else {
                            None
                        }
                    )
                }
                // TODO: analyze/test
                FieldIdent::Unnamed(_) => unreachable!(),
            };
            let replace_expr = match &field.ident {
                FieldIdent::Named(name) => {
                    let field_name = StringIdent(name.into()).named();
                    quote!(
                        *data = #ty::#variant_name {
                            #field_name: v
                        };
                    )
                }
                FieldIdent::Unnamed(0) => quote!(
                    *data = #ty::#variant_name (v);
                ),
                // TODO: analyze/test
                FieldIdent::Unnamed(_) => unreachable!(),
            };

            let quote = quote! {
                // TODO: rename into PartialPrism
                impl #impl_generics druid::Prism<
                    #ty#ty_generics,
                    #field_ty
                > for #twizzled_name::#variant_name
                #where_clause {

                    fn with<
                        #val_ty_par,
                        #func_ty_par: FnOnce(&#field_ty) -> #val_ty_par
                    > (
                        &self,
                        data: &#ty#ty_generics,
                        f: #func_ty_par
                    ) -> Option<#val_ty_par> {
                        #with_expr
                    }

                    fn with_mut<
                        #val_ty_par,
                        #func_ty_par: FnOnce(&mut #field_ty) -> #val_ty_par
                    > (
                        &self,
                        data: &mut #ty#ty_generics,
                        f: #func_ty_par,
                    ) -> Option<#val_ty_par> {
                        #with_expr
                    }
                }

                // TODO: move into another derivation
                impl #impl_generics druid::optics::Replace<
                    #ty#ty_generics,
                    #field_ty
                > for #twizzled_name::#variant_name
                #where_clause {
                    fn replace<'a>(
                        &self,
                        data: &'a mut #ty#ty_generics,
                        v: #field_ty
                    ) -> &'a mut #ty#ty_generics {
                        #replace_expr
                        data
                    }
                }
            };
            Ok(quote)
        })
        .collect::<Result<_, _>>()?;

    let associated_items = variants.iter().filter(|v| !v.attrs.ignore).map(|v| {
        let variant_name = &v.ident.named();
        let prism_variant_name = match v.attrs.prism_name_override.as_ref() {
            Some(name) => name.clone(),
            None => {
                if is_camel_case(&variant_name.to_string()) {
                    let temp_name = to_snake_case(&variant_name.to_string());
                    proc_macro2::Ident::new(&temp_name, proc_macro2::Span::call_site())
                } else {
                    return Err(syn::Error::new(
                        ty.span(),
                        "Prism implementations can only be derived from CamelCase variants",
                    ));
                }
            }
        };

        Ok(quote! {
            /// Prism for the corresponding variant
            pub const #prism_variant_name: #twizzled_name::#variant_name = #twizzled_name::#variant_name;
        })
    }).collect::<Result<Vec<_>, _>>()?;
    let associated_items = associated_items.iter();

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
