//! Builds the release binary and copies it with the name Tauri expects for a sidecar,
//! `swarmy-bevy-<target triple>`, i.e. into swarmy's assets directory:
//!
//! ```bash
//! cargo run --bin tauri-sidecar -- ../swarmy/assets
//! cargo run --bin tauri-sidecar -- ../swarmy/assets x86_64-pc-windows-gnu
//! ```
//!
//! A build script runs before the binary is compiled, so it cannot copy it, it only provides the
//! target triple of the host build.

use std::path::PathBuf;
use std::process::Command;

const BINARY: &str = "swarmy-bevy";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let destination = PathBuf::from(
        args.next()
            .ok_or("Usage: tauri-sidecar <destination directory> [target triple]")?,
    );
    let host = env!("SWARMY_BEVY_TARGET_TRIPLE");
    let target = args.next().unwrap_or_else(|| host.to_string());
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    // The dev features are not bundled. The target is always explicit, so the binary is in
    // target/<triple>/release also when .cargo/config.toml sets `build.target`, otherwise an older
    // binary in target/release could be copied.
    let mut build = Command::new(cargo);
    build.current_dir(&manifest_dir).args([
        "build",
        "--release",
        "--no-default-features",
        "--bin",
        BINARY,
        "--target",
        &target,
    ]);
    let release_dir = manifest_dir.join("target").join(&target).join("release");
    println!("Building {BINARY} for {target}");
    if !build.status()?.success() {
        return Err(format!("Building {BINARY} for {target} failed").into());
    }

    let extension = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let source = release_dir.join(format!("{BINARY}{extension}"));
    std::fs::create_dir_all(&destination)?;
    let sidecar = destination.join(format!("{BINARY}-{target}{extension}"));
    std::fs::copy(&source, &sidecar)?;
    println!(
        "Copied {} to {} ({} bytes)",
        source.display(),
        sidecar.display(),
        std::fs::metadata(&sidecar)?.len()
    );
    Ok(())
}
