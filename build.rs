//! Exports the target triple, the sidecar binary bundled by Tauri must be named
//! `swarmy-bevy-<target triple>`, see `src/bin/tauri-sidecar.rs`.

fn main() {
    let target = std::env::var("TARGET").expect("cargo sets TARGET for build scripts");
    println!("cargo:rustc-env=SWARMY_BEVY_TARGET_TRIPLE={target}");
    // The scene is embedded in the binary, see `src/embedded_assets.rs`.
    println!("cargo:rerun-if-changed=assets/swarmy-objects.gltf");
    println!("cargo:rerun-if-changed=assets/swarmy-objects.bin");
    println!("cargo:rerun-if-changed=build.rs");
}
