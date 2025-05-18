use magma_sys::*;
use quote::quote;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let wrapper_path = out_path.join("wrapper.cpp");
    fs::write(
        &wrapper_path,
        "#define VMA_IMPLEMENTATION\n#include \"vk_mem_alloc.h\"",
    )
    .unwrap();

    let mut build = cc::Build::new();

    #[cfg(not(debug_assertions))]
    build.define("NDEBUG", "").flag("-U_FORTIFY_SOURCE");
    #[cfg(debug_assertions)]
    build.flag("-O2");

    build
        .include("VulkanMemoryAllocator/include")
        .include("../vulkan-sys/Vulkan-Headers/include")
        .define("VMA_STATIC_VULKAN_FUNCTIONS", "0")
        .define("VMA_DYNAMIC_VULKAN_FUNCTIONS", "1")
        .file(&wrapper_path)
        .flag("-std=c++17")
        .flag("-Wno-missing-field-initializers")
        .flag("-Wno-unused-variable")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-unused-private-field")
        .flag("-Wno-reorder")
        .flag("-Wno-nullability-completeness")
        .cpp(true)
        .compile("vma");

    let bindings = bindgen::Builder::default()
        .header("VulkanMemoryAllocator/include/vk_mem_alloc.h")
        .clang_arg("-I../vulkan-sys/Vulkan-Headers/include")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .blocklist_type("Vk.*")
        .blocklist_type("PFN_vk.*")
        .blocklist_function("vk.*")
        .raw_line("use vulkan_sys::vk::*;")
        .parse_callbacks(Box::new(RenamerParseCallback::new("vk")))
        .parse_callbacks(Box::new(RenamerParseCallback::new("vma")))
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

    let structure_impls = {
        let mut builder = StructureBuilder::new();
        builder.visit_file(&file);
        builder.into_iter()
    };

    file.items.extend(structure_impls);

    fs::write(
        out_path.join("bindings.rs"),
        &format(&quote!(#file).to_string()),
    )
    .unwrap();
}
