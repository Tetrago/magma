use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use std::collections::HashSet;
use syn::parse_macro_input;

fn parse_attr(attr: &syn::Attribute) -> Option<(HashSet<String>, HashMap<String, syn::Expr>)> {
    let mut flags = HashSet::<String>::new();
    let mut values = HashMap::<String, syn::Expr>::new();

    attr.parse_nested_meta(|meta| {
        if let Some(path) = meta.path.get_ident() {
            if let Ok(value) = meta.value() {
                let expr: syn::Expr = value.parse()?;
                values.insert(path.to_string(), expr);
            } else {
                flags.insert(path.to_string());
            }
        }

        Ok(())
    })
    .ok();

    Some((flags, values))
}

fn extract_vec_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(path) = ty {
        if path.qself.is_none() {
            if let Some(segment) = path.path.segments.last() {
                if segment.ident == "Vec" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(ty)) = args.args.first() {
                            return Some(ty);
                        }
                    }
                }
            }
        }
    }

    None
}

fn make_setter(ident: &syn::Ident, ty: &syn::Type) -> proc_macro2::TokenStream {
    let vec_impl = extract_vec_inner_type(ty).map(|ty| {
        let extend_ident = syn::Ident::new(&format!("extend_{}", ident.to_string()), ident.span());
        let push_ident = syn::Ident::new(&format!("push_{}", ident.to_string()), ident.span());

        quote! {
            pub fn #extend_ident<I>(mut self, iter: I) -> Self
            where
                I: ::std::iter::IntoIterator<Item = #ty>, {
                self.#ident.extend(iter);
                self
            }

            pub fn #push_ident(mut self, item: #ty) -> Self {
                self.#ident.push(item);
                self
            }
        }
    });

    quote! {
        #vec_impl

        pub fn #ident(mut self, #ident: #ty) -> Self {
            self.#ident = #ident;
            self
        }
    }
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn builder_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;

    let build = if let Some((_, values)) = input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("builder"))
        .filter_map(parse_attr)
        .find(|(_, values)| values.contains_key("target"))
    {
        let target = &values["target"];

        Some(quote! {
            pub fn build(self) -> crate::Result<#target> {
                #target::new(self)
            }
        })
    } else {
        None
    };

    let fields = match input.data {
        syn::Data::Struct(ref data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => panic!("Only named fields are supported for builders"),
        },
        _ => panic!("Builder can only be derived for structs"),
    };

    let defaults = fields.iter().map(|field| {
        let name = &field.ident;

        if let Some((_, values)) = field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("builder"))
            .filter_map(parse_attr)
            .find(|(_, values)| values.contains_key("default"))
        {
            let expr = &values["default"];
            quote! { #name: #expr }
        } else {
            quote! { #name: ::std::default::Default::default() }
        }
    });

    let setters = fields
        .iter()
        .filter(|field| {
            !field
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("builder"))
                .filter_map(parse_attr)
                .any(|(flags, _)| flags.contains("None"))
        })
        .map(|field| make_setter(field.ident.as_ref().unwrap(), &field.ty));

    let output = quote! {
        impl Default for #name {
            fn default() -> Self {
                Self {
                    #(#defaults),*
                }
            }
        }

        impl #name {
            #build

            #(#setters)*
        }
    };

    TokenStream::from(output)
}

#[proc_macro_derive(Object, attributes(object))]
pub fn object_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;

    let builder = if let Some((_, values)) = input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("object"))
        .filter_map(parse_attr)
        .find(|(_, values)| values.contains_key("builder"))
    {
        let builder = &values["builder"];

        Some(quote! {
            pub fn builder() -> #builder {
                #builder::default()
            }
        })
    } else {
        None
    };

    let fields = match input.data {
        syn::Data::Struct(ref data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => panic!("Only named fields are supported for builders"),
        },
        _ => panic!("Builder can only be derived for structs"),
    };

    let handle = if let Some(field) = fields.iter().find(|field| {
        field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("object"))
            .filter_map(parse_attr)
            .any(|(flags, _)| flags.contains("handle"))
    }) {
        let name = &field.ident;
        let ty = &field.ty;

        Some(quote! {
            pub fn #name(&self) -> #ty {
                self.#name
            }
        })
    } else {
        None
    };

    let output = quote! {
        impl #name {
            #builder
            #handle
        }
    };

    TokenStream::from(output)
}
