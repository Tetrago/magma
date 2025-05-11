#![allow(unused_imports)]

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

#[cfg(feature = "example_shaders")]
fn compile_shader(src: &Path, dst: &Path, stage: &str) -> Result<(), String> {
    let output = Command::new("glslc")
        .arg(&format!("-fshader-stage={}", stage))
        .arg(src)
        .arg("-o")
        .arg(dst)
        .output()
        .expect("Failed to start glslc");

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

#[cfg(feature = "example_shaders")]
fn build_shaders() {
    use regex::Regex;
    use walkdir::WalkDir;

    let stage_regex = Regex::new(r"\.(\w+)\.glsl$").unwrap();

    let examples_dir = Path::new("examples");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("shaders");

    for entry in WalkDir::new(examples_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "glsl")
                .unwrap_or(false)
        })
    {
        let src_path = entry.path();
        let rel_path = src_path.strip_prefix(examples_dir).unwrap();
        let mut dst_path = out_dir.join(rel_path);
        dst_path.set_extension("spv");

        let stage = stage_regex.captures(src_path.to_str().unwrap()).unwrap()[1].to_owned();

        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        compile_shader(src_path, &dst_path, &stage).unwrap();

        println!("cargo:rerun-if-changed={}", src_path.display());
    }
}

fn main() {
    if cfg!(feature = "example_shaders") {
        #[cfg(feature = "example_shaders")]
        build_shaders();
    }
}
