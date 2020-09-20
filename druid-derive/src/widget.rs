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

use crate::attr::{FieldIdent, FieldKind, Fields, WidgetAttrs};
use crate::utils::*;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{spanned::Spanned, Data, GenericArgument, PathArguments, Type};

pub(crate) fn derive_widget_impl(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        Data::Struct(_) => derive_struct(&input),
        Data::Enum(e) => Err(syn::Error::new(
            e.enum_token.span(),
            "Widget implementations cannot be derived from enums",
        )),
        Data::Union(u) => Err(syn::Error::new(
            u.union_token.span(),
            "Widget implementations cannot be derived from unions",
        )),
    }
}

fn derive_struct(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ty = &input.ident;

    let twizzled_name = if is_camel_case(&ty.to_string()) {
        let temp_name = format!("{}_derived_widgets", to_snake_case(&ty.to_string()));
        proc_macro2::Ident::new(&temp_name, proc_macro2::Span::call_site())
    } else {
        return Err(syn::Error::new(
            ty.span(),
            "Widget implementations can only be derived from CamelCase types",
        ));
    };

    let fields = if let syn::Data::Struct(syn::DataStruct { fields, .. }) = &input.data {
        Fields::<WidgetAttrs>::parse_ast(fields)?
    } else {
        return Err(syn::Error::new(
            input.span(),
            "Widget implementations can only be derived from structs with named fields",
        ));
    };

    if fields.kind != FieldKind::Named {
        return Err(syn::Error::new(
            input.span(),
            "Widget implementations can only be derived from structs with named fields",
        ));
    }

    let meta_iter = fields.iter().filter(|f| f.attrs.meta);
    let count = meta_iter.count();

    if count == 0 {
        return Err(syn::Error::new(
            input.span(),
            "Widget implementations should have meta information",
        ));
    }

    if count > 1 {
        return Err(syn::Error::new(
            input.span(),
            "Widget implementations can't have more than one meta information",
        ));
    }

    let meta_field = fields.iter().find(|f| f.attrs.meta).unwrap();
    let meta_ident = if let FieldIdent::Named(ident) = &meta_field.ident {
        Ident::new(ident, Span::call_site())
    } else {
        return Err(syn::Error::new(
            input.span(),
            "Widget implementations can't have more than one meta information",
        ));
    };

    let data_ty = match &meta_field.ty {
        Type::Path(typepath) if typepath.qself.is_none() && typepath.path.segments.len() == 1 => {
            // Get the first segment of the path (there is only one, in fact: "Option"):
            let type_params = &typepath.path.segments.iter().next().unwrap().arguments;
            // It should have only on angle-bracketed param ("<String>"):
            let generic_arg = match type_params {
                PathArguments::AngleBracketed(params) => params.args.iter().next().unwrap(),
                _ => return Err(syn::Error::new(input.span(), "Invalid meta type")),
            };
            // This argument must be a type:
            match generic_arg {
                GenericArgument::Type(ty) => ty,
                _ => return Err(syn::Error::new(input.span(), "Invalid meta type")),
            }
        }
        _ => return Err(syn::Error::new(input.span(), "Invalid meta type")),
    };

    let expanded = quote! {
        mod #twizzled_name {
            use super::#ty;
            use druid::kurbo::{Point, Rect, Size};
            use druid::{
                BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
                UpdateCtx, Widget, WidgetPod,
            };

            impl Widget<#data_ty> for #ty {
                fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut #data_ty, env: &Env) {
                    if let Some(widget) = &mut self.#meta_ident.widget {
                        widget.event(ctx, event, data, env);
                    }
                }

                fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &#data_ty, env: &Env) {
                    if let LifeCycle::WidgetAdded = event {
                        self.#meta_ident.widget.replace(WidgetPod::new(self.build()).boxed());
                    }

                    if let Some(widget) = &mut self.#meta_ident.widget {
                        widget.lifecycle(ctx, event, data, env);
                    }
                }

                fn update(&mut self, ctx: &mut UpdateCtx, _: &#data_ty, data: &#data_ty, env: &Env) {
                    if let Some(widget) = &mut self.#meta_ident.widget {
                        widget.update(ctx, data, env);
                    }
                }

                fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &#data_ty, env: &Env) -> Size {
                    match &mut self.#meta_ident.widget {
                        Some(widget) => {
                            let size = widget.layout(ctx, &bc, data, env);
                            let rect = Rect::from_origin_size(Point::ORIGIN, size);
                            widget.set_layout_rect(ctx, data, env, rect);
                            size
                        }
                        None => Size::ZERO,
                    }
                }

                fn paint(&mut self, ctx: &mut PaintCtx, data: &#data_ty, env: &Env) {
                    if let Some(widget) = &mut self.#meta_ident.widget {
                        widget.paint(ctx, data, env);
                    }
                }
            }
        }

        pub use #twizzled_name::*;
    };

    Ok(expanded)
}
