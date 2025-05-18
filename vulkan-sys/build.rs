use magma_sys::*;
use quote::quote;
use std::env;
use std::fs;
use std::path::PathBuf;
use syn::parse_quote;
use syn::visit::Visit;

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
            pub fn result_to_string(result: Result) -> Option<&'static str> {
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

        let name = base_ident
            .to_string()
            .strip_prefix("cmd_")
            .unwrap()
            .to_string();
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
                handle: CommandBuffer
            }
        };

        let functions: Vec<_> = self.functions.iter().map(Self::transform).collect();

        let implement = syn::parse_quote! {
            impl CommandBufferInternal {
                #(#functions)*

                pub fn handle(&self) -> CommandBuffer {
                    self.handle
                }
            }
        };

        let cast = syn::parse_quote! {
            impl From<CommandBuffer> for CommandBufferInternal {
                fn from(value: CommandBuffer) -> Self {
                    Self { handle: value }
                }
            }
        };

        vec![structure, implement, cast]
    }
}

impl<'a> Visit<'a> for CommandCollector {
    fn visit_foreign_item_fn(&mut self, node: &'a syn::ForeignItemFn) {
        if node.sig.ident.to_string().starts_with("cmd_") {
            self.functions.push(node.sig.clone());
        }

        syn::visit::visit_foreign_item_fn(self, node);
    }
}

fn main() {
    println!("cargo:rustc-link-lib=vulkan");

    let bindings = bindgen::Builder::default()
        .header("Vulkan-Headers/include/vulkan/vulkan.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .parse_callbacks(Box::new(RenamerParseCallback::new("vk")))
        .must_use_type("VkResult")
        .prepend_enum_name(false)
        .formatter(bindgen::Formatter::None)
        .layout_tests(false)
        .generate_comments(false)
        .generate()
        .expect("Unable to generate bindings")
        .to_string();

    let mut file: syn::File = syn::parse_str(&bindings).unwrap();
    StructFieldRenamer::new().visit_file_mut(&mut file);

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
    fs::write(
        out_path.join("bindings.rs"),
        &format(&quote!(#file).to_string()),
    )
    .unwrap();
    fs::write(
        out_path.join("internal.rs"),
        &format(&quote!(#(#internal_command_buffer)*).to_string()),
    )
    .unwrap();
}
