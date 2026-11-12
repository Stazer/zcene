use glob::glob;
use std::env::var;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/bin");

    if var("CARGO_CFG_TARGET_OS").as_deref() != Ok("none") {
        return Ok(());
    }

    let out_dir = var("OUT_DIR")?;
    let package = var("CARGO_PKG_NAME")?;
    let architecture = var("CARGO_CFG_TARGET_ARCH")?;
    let manifest_directory = PathBuf::from(var("CARGO_MANIFEST_DIR")?);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let zcene_directory = root.join("zcene");

    let images = root.join("target").join(&architecture).join("images").join(&package);
    println!("cargo:rustc-env=EXAMPLE_IMAGES={}", images.display());

    let kernel_script = zcene_directory.join("linker.ld");
    let program_script = zcene_directory.join("unprivileged.ld");
    println!("cargo:rerun-if-changed={}", kernel_script.display());
    println!("cargo:rerun-if-changed={}", program_script.display());

    println!("cargo:rustc-link-arg-bin={package}=--script={}", kernel_script.display());
    assemble_sources(&out_dir, &package, &zcene_directory.join("src"))?;

    for entry in glob(&format!("{}/src/bin/*.rs", manifest_directory.display()))? {
        let path = entry?;
        let program = path.file_stem().ok_or("Expect stem")?.display().to_string();

        println!("cargo:rustc-link-arg-bin={program}=--script={}", program_script.display());
    }

    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////

fn assemble_sources(
    out_dir: &str,
    binary: &str,
    source_directory: &Path,
) -> Result<(), Box<dyn Error>> {
    let glob_pattern = format!("{}/**/*.asm", source_directory.display());

    for entry in glob(&glob_pattern)? {
        let path = entry?;

        let object_name = format!("{}.o", path.file_stem().ok_or("Expect stem")?.display());

        let status = Command::new("nasm")
            .current_dir(out_dir)
            .args(["-felf64", "-g", "-Fdwarf", "-o", &object_name])
            .arg(&path)
            .status()?;

        if !status.success() {
            return Err(format!("nasm failed for {}", path.display()).into());
        }

        println!("cargo:rerun-if-changed={}", path.display());
        println!("cargo:rustc-link-arg-bin={binary}={out_dir}/{object_name}");
    }

    Ok(())
}
