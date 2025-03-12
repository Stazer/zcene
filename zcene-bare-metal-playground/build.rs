#![feature(exit_status_error)]

////////////////////////////////////////////////////////////////////////////////////////////////////

use glob::glob;
use std::env::var;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;

////////////////////////////////////////////////////////////////////////////////////////////////////

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = var("OUT_DIR")?;
    let manifest_dir = PathBuf::from(var("CARGO_MANIFEST_DIR")?);

    println!("cargo:rerun-if-changed=build.rs");
    println!(
        "cargo:rustc-link-arg-bins=--script={}/../zcene-bare-metal/linker.ld",
        manifest_dir.display(),
    );

    let source_dir = manifest_dir.join("src");

    let glob_pattern = format!("{}/**/*.asm", source_dir.display(),);

    for entry in glob(&glob_pattern)? {
        let path = entry?;

        let object_name = &format!("{}.o", path.file_stem().ok_or("Expect stem")?.display());

        Command::new("nasm")
            .current_dir(&out_dir)
            .args(&[
                "-felf64",
                "-g",
                "-Fdwarf",
                "-o",
                object_name,
                &path.display().to_string(),
            ])
            .status()?
            .exit_ok()?;

        println!("cargo:rerun-if-changed={}", path.display());
        println!("cargo:rustc-link-arg-bins={}/{}", out_dir, object_name);
    }

    Ok(())
}
