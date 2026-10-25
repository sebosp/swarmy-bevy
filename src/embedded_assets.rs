//! The assets created with Blender and Skein are embedded in the binary, so that the binary
//! bundled with swarmy, which has no `assets` directory next to it, can load them.

use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::prelude::*;
use std::path::{Path, PathBuf};

/// The Blender scene with the materials of the map objects.
pub const SWARMY_OBJECTS_GLTF: &str = "embedded://swarmy_bevy/swarmy-objects.gltf";

/// The files of the scene, the gltf references the bin by a relative path, so both are embedded
/// in the same directory.
const EMBEDDED_FILES: [(&str, &[u8]); 2] = [
    (
        "swarmy-objects.gltf",
        include_bytes!("../assets/swarmy-objects.gltf"),
    ),
    (
        "swarmy-objects.bin",
        include_bytes!("../assets/swarmy-objects.bin"),
    ),
];

/// Registers the embedded assets, must be added after the `AssetPlugin` (`DefaultPlugins`).
pub struct EmbeddedAssetsPlugin;

impl Plugin for EmbeddedAssetsPlugin {
    fn build(&self, app: &mut App) {
        let registry = app.world().resource::<EmbeddedAssetRegistry>();
        for (name, bytes) in EMBEDDED_FILES {
            registry.insert_asset(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("assets")
                    .join(name),
                &Path::new("swarmy_bevy").join(name),
                bytes,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A checkout without Git LFS has pointer files instead of the scene.
    #[test]
    fn it_embeds_the_scene_not_the_lfs_pointers() {
        for (name, bytes) in EMBEDDED_FILES {
            assert!(
                !bytes.starts_with(b"version https://git-lfs"),
                "{name} is a Git LFS pointer, fetch the LFS files"
            );
        }
        let gltf = std::str::from_utf8(EMBEDDED_FILES[0].1).unwrap();
        assert!(gltf.trim_start().starts_with('{') && gltf.contains("\"asset\""));
    }
}
