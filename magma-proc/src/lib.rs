use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

fn attr_by_name(
    derive_attr: &str,
    name: &str,
) -> Box<dyn Fn(&syn::Attribute) -> Option<syn::Expr>> {
    let derive_attr = derive_attr.to_owned();
    let name = name.to_owned();

    Box::new(move |attr| {
        if attr.path().is_ident(&derive_attr) {
            let mut result = None;

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(&name) {
                    let value: syn::Expr = meta.value()?.parse()?;
                    result = Some(value);
                    Ok(())
                } else {
                    Err(meta.error("unknown attribute"))
                }
            })
            .unwrap();

            result
        } else {
            None
        }
    })
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

#[proc_macro_derive(Builder, attributes(builder, skip))]
pub fn builder_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;

    let build = if let Some(target) = input
        .attrs
        .iter()
        .find_map(attr_by_name("builder", "target"))
    {
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

        if let Some(expr) = field
            .attrs
            .iter()
            .find_map(attr_by_name("builder", "default"))
        {
            quote! { #name: #expr }
        } else {
            quote! { #name: ::std::default::Default::default() }
        }
    });

    let setters = fields
        .iter()
        .filter(|field| !field.attrs.iter().any(|attr| attr.path().is_ident("skip")))
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

#[proc_macro_derive(Object, attributes(handle, object))]
pub fn object_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;

    let builder = if let Some(builder) = input
        .attrs
        .iter()
        .find_map(attr_by_name("object", "builder"))
    {
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
            .any(|attr| attr.path().is_ident("handle"))
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
