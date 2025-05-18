use regex::Regex;
use std::io::prelude::*;
use std::process::Command;
use std::process::Stdio;
use syn::parse_quote;
pub use syn::visit::Visit;
pub use syn::visit_mut::VisitMut;

pub fn format(str: &str) -> String {
    let mut rustfmt = Command::new("rustfmt")
        .arg("--emit")
        .arg("stdout")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to format generated bindings");

    rustfmt
        .stdin
        .as_mut()
        .unwrap()
        .write_all(str.as_bytes())
        .unwrap();

    String::from_utf8(rustfmt.wait_with_output().unwrap().stdout).unwrap()
}

pub fn to_snake_case(str: &str) -> String {
    let mut result = String::with_capacity(str.len() * 2);

    for (i, c) in str.chars().enumerate() {
        if c.is_uppercase() {
            if i != 0 {
                result.push('_');
            }

            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }

    result
}

#[derive(Debug)]
pub struct RenamerParseCallback {
    function_namespace: String,
    type_namespace: String,
    constant_namespace: String,
}

impl RenamerParseCallback {
    pub fn new(namespace: &str) -> Self {
        Self {
            function_namespace: namespace.to_lowercase(),
            type_namespace: format!(
                "{}{}",
                namespace.chars().next().unwrap().to_ascii_uppercase(),
                namespace[1..].to_lowercase()
            ),
            constant_namespace: format!("{}_", namespace.to_uppercase()),
        }
    }
}

impl bindgen::callbacks::ParseCallbacks for RenamerParseCallback {
    fn item_name(&self, name: &str) -> Option<String> {
        Some(if let Some(name) = name.strip_prefix("PFN_") {
            format!("PFN_{}", name.to_owned())
        } else if let Some(name) = name.strip_prefix(&self.function_namespace) {
            // Correctly handle function suffixes, preventing "_k_h_r"
            let name = name
                .rfind(|c: char| !c.is_uppercase())
                .filter(|&index| index != name.len() - 1)
                .map(|index| {
                    format!(
                        "{}_{}",
                        name[0..=index].to_string(),
                        name[index + 1..].to_lowercase()
                    )
                })
                .unwrap_or(name.to_owned());

            to_snake_case(&name)
        } else if let Some(name) = name.strip_prefix(&self.constant_namespace) {
            name.to_owned()
        } else if let Some(name) = name.strip_prefix(&self.type_namespace) {
            name.to_owned()
        } else {
            return None;
        })
    }

    fn enum_variant_name(
        &self,
        _enum_name: Option<&str>,
        name: &str,
        _variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<String> {
        name.strip_prefix(&self.constant_namespace)
            .map(str::to_owned)
    }
}

pub struct StructureBuilder {
    regex: Regex,
    items: Vec<syn::Item>,
}

impl StructureBuilder {
    pub fn new() -> Self {
        Self {
            regex: Regex::new(
                r"[0-9]+(tnI|tniU)|61taolF|VN|tiB[0-9]+|RDH|6X01ABGR|(?<vulkan>[0-9]+)nakluV|[A-Z]+|[a-z]+[A-Z]|[0-9]+",
            )
            .unwrap(),
            items: Vec::new(),
        }
    }
}

impl IntoIterator for StructureBuilder {
    type Item = syn::Item;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> Visit<'a> for StructureBuilder {
    fn visit_item_struct(&mut self, node: &'a syn::ItemStruct) {
        let name = syn::Ident::new(&node.ident.to_string(), proc_macro2::Span::call_site());

        if node.generics.params.is_empty() {
            let item: syn::ItemImpl = if node.fields.iter().any(|field| {
                field
                    .ident
                    .as_ref()
                    .map(|ident| ident.to_string() == "s_type")
                    .unwrap_or(false)
                    && !node.ident.to_string().ends_with("Structure")
            }) {
                let structure_type = syn::Ident::new(
                    &format!("STRUCTURE_TYPE{}", {
                        let name = &node
                            .ident
                            .to_string()
                            .chars()
                            .collect::<Vec<char>>()
                            .iter()
                            .rev()
                            .collect::<String>();

                        self.regex
                            .replace_all(name, |captures: &regex::Captures| {
                                if let Some(version) = captures.name("vulkan") {
                                    let mut result = String::new();

                                    for c in version.as_str().chars() {
                                        result.push(c);
                                        result.push('_');
                                    }

                                    format!("{}NAKLUV_", result)
                                } else {
                                    format!("{}_", captures[0].to_uppercase())
                                }
                            })
                            .to_string()
                            .chars()
                            .rev()
                            .collect::<String>()
                    }),
                    proc_macro2::Span::call_site(),
                );

                parse_quote! {
                    impl Default for #name {
                        fn default() -> Self {
                            Self {
                                s_type: #structure_type,
                                ..unsafe { ::std::mem::zeroed() }
                            }
                        }
                    }
                }
            } else {
                parse_quote! {
                    impl Default for #name {
                        fn default() -> Self {
                            Self {
                                ..unsafe { ::std::mem::zeroed() }
                            }
                        }
                    }
                }
            };

            self.items.push(syn::Item::Impl(item));

            let mut item: syn::ItemImpl = parse_quote! {
                impl #name { }
            };

            item.items.extend(
                node.fields
                    .iter()
                    .filter(|field| {
                        field
                            .ident
                            .as_ref()
                            .map(|ident| ident.to_string() != "s_type")
                            .unwrap_or(field.ident.is_some())
                    })
                    .map(|field| {
                        let name = syn::Ident::new(
                            &field.ident.as_ref().unwrap().to_string(),
                            proc_macro2::Span::call_site(),
                        );
                        let ty = &field.ty;

                        parse_quote! {
                            pub fn #name(mut self, #name: #ty) -> Self {
                                self.#name = #name;
                                self
                            }
                        }
                    }),
            );

            self.items.push(syn::Item::Impl(item));
        }

        syn::visit::visit_item_struct(self, node);
    }
}

pub struct StructFieldRenamer {
    regex: Regex,
}

impl StructFieldRenamer {
    pub fn new() -> Self {
        Self {
            regex: Regex::new(r"(pp?_)?(\w+)").unwrap(),
        }
    }
}

impl VisitMut for StructFieldRenamer {
    fn visit_item_struct_mut(&mut self, node: &mut syn::ItemStruct) {
        for field in node
            .fields
            .iter_mut()
            .filter_map(|field| field.ident.as_mut())
        {
            let name = to_snake_case(&field.to_string());
            let name = self.regex.replace(&name, "$2");

            // There are pGeometries and ppGeometries fields in the same struct
            if name != "geometries" {
                *field = syn::Ident::new(&name, field.span());
            }
        }

        syn::visit_mut::visit_item_struct_mut(self, node);
    }
}
