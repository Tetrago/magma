use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use regex::Regex;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use syn::parse_quote;
use syn::visit::Visit;
use syn::visit_mut::VisitMut;

macro_rules! rename {
    ($str:expr, { $($pattern:literal => $expr:expr),+ $(,)? }) => {{
        let result = $str.to_owned();
        $(let result = regex::Regex::new($pattern).unwrap().replace_all(&result, $expr);)+
        result.into_owned()
    }};
    ($str:expr, $pattern:literal => $expr:expr) => {
        rename!($str, { $pattern => $expr })
    }
}

fn format(str: &str) -> String {
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

fn to_snake_case(str: &str) -> String {
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

fn save_links(str: &str) -> String {
    rename!(str, r#"#\[link_name = "([A-Za-z0-9_]+)"\]"# => |captures: &regex::Captures| format!("__LINK_{}__", &captures[1]))
}

fn restore_links(str: &str) -> String {
    rename!(str, r"__LINK_(\w+)__" => |captures: &regex::Captures| format!(r#"#[link_name = "{}"]"#, &captures[1]))
}

fn substitute_names(str: &str) -> String {
    use regex::Captures;

    rename!(str, {
        r"\b([A-Z]+)_(vk[A-Z][A-Za-z]+)\b" => |captures: &Captures| {
            format!("{}::{}", captures[1].to_lowercase(), &captures[2])
        },
        r"\bvk([A-Z][A-Za-z]+[a-z])([A-Z]+)\b" => |captures: &Captures| {
            format!("{}_{}", to_snake_case(&captures[1]), captures[2].to_lowercase())
        },
        r"\bvk([A-Z]\w+)\b" => |captures: &Captures| to_snake_case(&captures[1]),
        r"\bV(k|K_)([A-Z]\w+)(?<str>\\0)?\b" => |captures: &Captures| {
            if captures.name("str").is_some() {
                captures[0].to_string()
            } else {
                captures[2].to_string()
            }
        },
        r"\b([A-Z][a-z]+)+([A-Z]+)?_VK_([A-Z0-9_]+)\b" => "$3",
    })
}

fn convert_cases(str: &str) -> String {
    rename!(str, {
        r"\b([a-z][A-Za-z0-9_]+)\b" => |captures: &regex::Captures| to_snake_case(&captures[1]),
        r"\bpp?_(\w+)\b" => |captures: &regex::Captures| {
            if &captures[1] != "geometries" {
                captures[1].to_string()
            } else {
                captures[0].to_string()
            }
        }
    })
}

fn extract_pfn_types(file: &mut syn::File) {
    let mut items = Vec::new();

    let mut i = 0;
    while i < file.items.len() {
        match &file.items[i] {
            syn::Item::Type(ty) if ty.ident.to_string().starts_with("PFN_") => {
                let mut item = file.items.remove(i);

                if let syn::Item::Type(ty) = &mut item {
                    if let Some(ident) = ty.ident.to_string().strip_prefix("PFN_") {
                        ty.ident = syn::Ident::new(&substitute_names(ident), ty.ident.span());
                    }
                }

                items.push(item);
            }
            _ => i += 1,
        };
    }

    items.push(syn::parse_quote!(
        use super::*;
    ));

    let pfn_mod = syn::ItemMod {
        attrs: vec![],
        vis: syn::Visibility::Public(syn::token::Pub::default()),
        unsafety: None,
        mod_token: syn::token::Mod::default(),
        ident: format_ident!("pfn"),
        content: Some((syn::token::Brace::default(), items)),
        semi: None,
    };

    file.items.push(syn::Item::Mod(pfn_mod));
}

struct Linker;

impl VisitMut for Linker {
    fn visit_item_foreign_mod_mut(&mut self, item: &mut syn::ItemForeignMod) {
        if item
            .abi
            .name
            .as_ref()
            .map(|abi| abi.value() == "C")
            .unwrap_or(false)
        {
            for item in &mut item.items {
                if let syn::ForeignItem::Fn(func) = item {
                    let link_name = func.sig.ident.to_string();

                    func.attrs.push(parse_quote! {
                        #[link_name = #link_name]
                    });
                }
            }
        }

        syn::visit_mut::visit_item_foreign_mod_mut(self, item);
    }
}

struct ResultMapper {
    arms: Vec<syn::Arm>,
}

impl ResultMapper {
    pub fn new() -> Self {
        Self { arms: Vec::new() }
    }

    pub fn build(&self) -> syn::ItemFn {
        let mut arms = self.arms.clone();
        arms.push(parse_quote! { _ => None });

        let mtch = syn::Expr::Match(syn::ExprMatch {
            attrs: vec![],
            match_token: Default::default(),
            expr: Box::new(parse_quote! { result }),
            brace_token: Default::default(),
            arms,
        });

        parse_quote! {
            pub fn result_to_string(result: VkResult) -> Option<&'static str> {
                #mtch
            }
        }
    }
}

impl<'a> Visit<'a> for ResultMapper {
    fn visit_item_const(&mut self, item: &'a syn::ItemConst) {
        if let Some(ident) = item.ident.to_string().strip_prefix("VkResult_VK_") {
            let expr = &item.expr;
            self.arms.push(parse_quote! { #expr => Some(#ident) });
        }

        syn::visit::visit_item_const(self, item);
    }
}

struct StructureBuilder {
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
                    .map(|ident| ident.to_string() == "sType")
                    .unwrap_or(false)
                    && !node.ident.to_string().ends_with("Structure")
            }) {
                let structure_type = syn::Ident::new(
                    &format!("StructureType_VK_STRUCTURE_TYPE{}", {
                        let name = &node
                            .ident
                            .to_string()
                            .chars()
                            .skip(2)
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
                                sType: #structure_type,
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
                            .map(|ident| ident.to_string() != "sType")
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

struct CommandCollector {
    functions: Vec<syn::Signature>,
}

impl CommandCollector {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }

    fn transform(sig: &syn::Signature) -> syn::ImplItemFn {
        let base_ident = &sig.ident;

        let name = base_ident.to_string().replace("Cmd", "");
        let ident = syn::Ident::new(&name, sig.ident.span());

        let args: Vec<_> = sig.inputs.iter().skip(1).collect();
        let call_idents: Vec<_> = args
            .iter()
            .map(|arg| {
                if let syn::FnArg::Typed(syn::PatType { pat, .. }) = arg {
                    quote! {#pat}
                } else {
                    quote! {}
                }
            })
            .collect();

        syn::parse_quote! {
            pub unsafe fn #ident(&mut self, #(#args),*) {
                #base_ident(self.handle, #(#call_idents),*);
            }
        }
    }

    pub fn build(&self) -> Vec<syn::Item> {
        let structure = syn::parse_quote! {
            pub struct CommandBufferInternal {
                handle: VkCommandBuffer
            }
        };

        let functions: Vec<_> = self.functions.iter().map(Self::transform).collect();

        let implement = syn::parse_quote! {
            impl CommandBufferInternal {
                #(#functions)*

                pub fn handle(&self) -> VkCommandBuffer {
                    self.handle
                }
            }
        };

        let cast = syn::parse_quote! {
            impl From<VkCommandBuffer> for CommandBufferInternal {
                fn from(value: VkCommandBuffer) -> Self {
                    Self { handle: value }
                }
            }
        };

        vec![structure, implement, cast]
    }
}

impl<'a> Visit<'a> for CommandCollector {
    fn visit_foreign_item_fn(&mut self, node: &'a syn::ForeignItemFn) {
        if node.sig.ident.to_string().starts_with("vkCmd") {
            self.functions.push(node.sig.clone());
        }

        syn::visit::visit_foreign_item_fn(self, node);
    }
}

fn finalize(token_stream: TokenStream) -> String {
    restore_links(&convert_cases(&substitute_names(&save_links(&format(
        &token_stream.to_string(),
    )))))
}

fn main() {
    println!("cargo:rustc-link-lib=vulkan");

    let bindings = bindgen::Builder::default()
        .header("Vulkan-Headers/include/vulkan/vulkan.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .must_use_type("VkResult")
        .formatter(bindgen::Formatter::None)
        .generate()
        .expect("Unable to generate bindings")
        .to_string();

    let mut file: syn::File = syn::parse_str(&bindings).unwrap();

    extract_pfn_types(&mut file);
    (Linker).visit_file_mut(&mut file);

    let internal_command_buffer = {
        let mut commands = CommandCollector::new();
        commands.visit_file(&file);
        commands.build()
    };

    let result_to_string = {
        let mut result_mapper = ResultMapper::new();
        result_mapper.visit_file(&file);
        syn::Item::Fn(result_mapper.build())
    };

    let structure_impls = {
        let mut builder = StructureBuilder::new();
        builder.visit_file(&file);
        builder.into_iter()
    };

    file.items.push(result_to_string);
    file.items.extend(structure_impls);

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_path.join("bindings.rs"), &finalize(quote!(#file))).unwrap();
    fs::write(
        out_path.join("internal.rs"),
        &finalize(quote!(#(#internal_command_buffer)*)),
    )
    .unwrap();
}
