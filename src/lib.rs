use bevy::gltf::GltfMaterial;
use bevy::prelude::*;
pub mod error;
pub use error::*;

pub use s2protocol::cache_handles::document_header::*;
pub use s2protocol::cache_handles::map::coords::*;
pub use s2protocol::cache_handles::map_info::*;
pub use s2protocol::cache_handles::t3_height_map::*;

pub mod cache_objects;
pub mod cli;
pub mod document_header;
pub mod embedded_assets;
pub mod map_info;
pub mod map_layers;
pub mod replay;
pub mod screenshot;
pub mod siege_spots;
pub mod swarmy_feathers;
pub mod t3_height_map;
pub mod t3_terrain;
pub mod utils;

pub mod map_plugin;

pub const MAP_SCALE_FACTOR: f32 = 0.1;

// Holds the scene handle
#[derive(Resource)]
pub struct MapScene(pub Handle<Gltf>);

/// Show some text if there's a current action
#[derive(Default, Component, Reflect, Clone)]
pub struct SelectedObjectName(pub String);

/// From gltf_pbr/src/gltf.rs, temp fix because I don't know how to load these materials...
/// Converts a [`GltfMaterial`] to a [`StandardMaterial`]
pub fn standard_material_from_gltf_material(material: &GltfMaterial) -> StandardMaterial {
    StandardMaterial {
        base_color: material.base_color,
        base_color_channel: material.base_color_channel.clone(),
        base_color_texture: material.base_color_texture.clone(),
        emissive: material.emissive,
        emissive_channel: material.emissive_channel.clone(),
        emissive_texture: material.emissive_texture.clone(),
        perceptual_roughness: material.perceptual_roughness,
        metallic: material.metallic,
        metallic_roughness_channel: material.metallic_roughness_channel.clone(),
        metallic_roughness_texture: material.metallic_roughness_texture.clone(),
        reflectance: material.reflectance,
        specular_tint: material.specular_tint,
        specular_transmission: material.specular_transmission,
        thickness: material.thickness,
        ior: material.ior,
        attenuation_distance: material.attenuation_distance,
        attenuation_color: material.attenuation_color,
        normal_map_channel: material.normal_map_channel.clone(),
        normal_map_texture: material.normal_map_texture.clone(),
        occlusion_channel: material.occlusion_channel.clone(),
        occlusion_texture: material.occlusion_texture.clone(),
        clearcoat: material.clearcoat,
        clearcoat_perceptual_roughness: material.clearcoat_perceptual_roughness,
        anisotropy_strength: material.anisotropy_strength,
        anisotropy_rotation: material.anisotropy_rotation,
        double_sided: material.double_sided,
        cull_mode: material.cull_mode,
        unlit: material.unlit,
        alpha_mode: material.alpha_mode,
        uv_transform: material.uv_transform,
        ..Default::default()
    }
}
