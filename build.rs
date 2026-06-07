use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    // Re-run this build script if assets changes
    println!("cargo:rerun-if-changed=assets");

    let out_dir = env::var("OUT_DIR").unwrap();
    // OUT_DIR is target/{profile}/build/{crate_name}-{hash}/out
    // We go up 3 levels to reach target/{profile}
    let out_path = Path::new(&out_dir);
    if let Some(target_dir) = out_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
    {
        let dest_assets = target_dir.join("assets");
        let src_assets = Path::new("assets");

        if src_assets.exists() {
            let _ = copy_dir_all(src_assets, &dest_assets);
        }
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
