use super::field_attr::FieldIdent;
use super::lens::{is_camel_case, to_snake_case};
use super::variant_attr::{StringIdent, Variants};
use quote::quote;
use syn::{spanned::Spanned, Data};

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
        Variants::parse_ast(variants)?
    } else {
        return Err(syn::Error::new(
            input.span(),
            "Prisms implementations can only be derived from .. (TODO)",
        ));
    };

    let twizzled_name = if is_camel_case(&ty.to_string()) {
        let temp_name = format!("{}_derived_prisms", to_snake_case(&ty.to_string()));
        proc_macro2::Ident::new(&temp_name, proc_macro2::Span::call_site())
    } else {
        return Err(syn::Error::new(
            ty.span(),
            "Prisms implementations can only be derived from CamelCase types",
        ));
    };

    // Define prisms types for each variant
    let defs = variants.iter().map(|v| {
        let variant_name = &v.ident.named();

        quote! {
            /// Prism for the variant on (the enum)
            #[allow(non_camel_case_types)]
            #[derive(Debug, Copy, Clone)]
            pub struct #variant_name;
        }
    });

    let impls = variants.iter().map(|v| {
        let variant_name = &v.ident.named();
        let field_ty = &v.field.ty;
        let (
            field_name,
            original_name
         ) = match &v.field.ident {
            FieldIdent::Named(name) => 
                (StringIdent(name.into()).named(), true)
            ,
            FieldIdent::Unnamed(0) => 
                (StringIdent("inner".into()).named(), false)
            ,
            FieldIdent::Unnamed(_) => unreachable!(),
        };

        if original_name {
            quote! {
                impl druid::Prism<#ty, #field_ty> for #twizzled_name::#variant_name {
    
                    fn with_raw<V, F: FnOnce(Option<&#field_ty>) -> Option<V>>(&self, data: &#ty, f: F) -> Option<V> {
                        let inner = if let #ty::#variant_name { #field_name } = data {
                            Some(#field_name)
                        } else {
                            None
                        };
                        f(inner)
                    }
    
                    fn with_raw_mut<V, F: FnOnce(Option<&mut #field_ty>) -> Option<V>>(
                        &self,
                        data: &mut #ty,
                        f: F,
                    ) -> Option<V> {
                        let inner = if let #ty::#variant_name { #field_name } = data {
                            Some(#field_name)
                        } else {
                            None
                        };
                        f(inner)
                    }
                }
            }
        } else {
            quote! {
                impl druid::Prism<#ty, #field_ty> for #twizzled_name::#variant_name {
    
                    fn with_raw<V, F: FnOnce(Option<&#field_ty>) -> Option<V>>(&self, data: &#ty, f: F) -> Option<V> {
                        let inner = if let #ty::#variant_name (#field_name) = data {
                            Some(#field_name)
                        } else {
                            None
                        };
                        f(inner)
                    }
    
                    fn with_raw_mut<V, F: FnOnce(Option<&mut #field_ty>) -> Option<V>>(
                        &self,
                        data: &mut #ty,
                        f: F,
                    ) -> Option<V> {
                        let inner = if let #ty::#variant_name(inner) = data {
                            Some(inner)
                        } else {
                            None
                        };
                        f(inner)
                    }
                }
            }

        }
    });

    let associated_items = variants.iter().map(|v| {
        let variant_name = &v.ident.named();
        let variant_twizzled_name = if is_camel_case(&variant_name.to_string()) {
            let temp_name = to_snake_case(&variant_name.to_string());
            proc_macro2::Ident::new(&temp_name, proc_macro2::Span::call_site())
        } else {
            return Err(syn::Error::new(
                ty.span(),
                "Prisms implementations can only be derived from CamelCase variants",
            ));
        };
        let prism_variant_name = &variant_twizzled_name;

        Ok(quote! {
            /// Prism for the corresponding variant
            pub const #prism_variant_name: #twizzled_name::#variant_name = #twizzled_name::#variant_name;
        })
    }).collect::<Result<Vec<_>, _>>()?;
    let associated_items = associated_items.iter();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

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
    // let exp = format!("{}", &expanded);
    // println!("{}", exp);

    Ok(expanded)
}
