use bootloader::DiskImageBuilder;
use std::env::var;
use std::error::Error;
use std::path::PathBuf;

////////////////////////////////////////////////////////////////////////////////////////////////////

fn main() -> Result<(), Box<dyn Error>> {
    let profile = var("PROFILE")?;
    let arch = var("CARGO_CFG_TARGET_ARCH")?;

    let target_path = PathBuf::from("..").join("target").join(arch).join(profile);

    println!("cargo:rerun-if-changed={}", target_path.display());

    let image_builder = DiskImageBuilder::new(target_path.join("zcene-bare-metal-playground"));
    image_builder
        .create_uefi_image(&target_path.join("zcene-bare-metal-playground-x86_64-uefi.img"))?;
    image_builder
        .create_bios_image(&target_path.join("zcene-bare-metal-playground-x86_64-bios.img"))?;

    Ok(())
}
