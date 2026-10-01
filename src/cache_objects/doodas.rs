//! Doodas defined in the Object file.

use crate::MAP_SCALE_FACTOR;
use crate::MapScene;
use crate::cache_objects::PlacedObjectsResource;
use crate::standard_material_from_gltf_material;
use crate::swarmy_feathers::DisplayInfoOnClick;
use crate::swarmy_feathers::update_info_on_click;
use bevy::gltf::GltfMaterial;
use bevy::prelude::*;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
pub struct ObjectDoodadComponent;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct ShadowPlatformRampMaterial;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct UnknownDoodadMaterial;

// Testing a Dooda.
pub const SHADOW_PLATFORM_RAMP_SIZE: f32 = 1.0;

pub fn load_object_doodas(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    placed_objects: Res<PlacedObjectsResource>,
    map_scene: Res<MapScene>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_materials: Res<Assets<GltfMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(gltf) = gltf_assets.get(&map_scene.0) else {
        return;
    };
    // An example doodad name observed in the Objects file.
    let shadow_platform_ramp_mesh = Mesh3d(meshes.add(Cuboid::new(
        SHADOW_PLATFORM_RAMP_SIZE * MAP_SCALE_FACTOR,
        SHADOW_PLATFORM_RAMP_SIZE * MAP_SCALE_FACTOR,
        SHADOW_PLATFORM_RAMP_SIZE * MAP_SCALE_FACTOR,
    )));
    let unknown_object_mesh = Mesh3d(meshes.add(Cuboid::new(
        SHADOW_PLATFORM_RAMP_SIZE * MAP_SCALE_FACTOR,
        SHADOW_PLATFORM_RAMP_SIZE * MAP_SCALE_FACTOR,
        SHADOW_PLATFORM_RAMP_SIZE * MAP_SCALE_FACTOR,
    )));
    // Id="1231" Position="118.1433,8.0437,6.9763" Scale="1,1,1" Type="Shadow_Platform_Ramp"
    for (doodad_ith, doodad) in placed_objects.doodas.iter().enumerate() {
        // These are objects in the map, decorations, animation references, etc.
        let unit_pos: Vec<f32> = match doodad.position {
            Some(ref pos) => pos
                .split(",")
                .filter_map(|x| x.parse::<f32>().ok())
                .collect(),
            None => continue,
        };
        if unit_pos.len() != 3 {
            tracing::error!(
                "Unexpected number of tokens for unit position typed: {}",
                doodad.kind
            );
            continue;
        }
        let x = unit_pos[0];
        let y = unit_pos[1];

        let Some(unit_handle) = gltf.named_materials.get(doodad.kind.as_str()) else {
            tracing::warn!(
                "Unhandled Skein GLTF named_material Doodad: {}",
                doodad.kind
            );
            continue;
        };
        let Some(unit_gltf_material) = gltf_materials.get(unit_handle.id()) else {
            tracing::warn!("Unhandled Skein GLTF gltf_material Doodad: {}", doodad.kind);
            continue;
        };
        let dooda_material = MeshMaterial3d(
            materials.add(standard_material_from_gltf_material(&unit_gltf_material)),
        );

        let shadow_platform_ramp_transform = Transform::from_xyz(
            MAP_SCALE_FACTOR * y,
            SHADOW_PLATFORM_RAMP_SIZE * MAP_SCALE_FACTOR,
            MAP_SCALE_FACTOR * x,
        );
        match doodad.kind.as_str() {
            "Shadow_Platform_Ramp" => commands
                .spawn((
                    ShadowPlatformRampMaterial,
                    ObjectDoodadComponent,
                    DisplayInfoOnClick,
                    Name(format!("{}:{}:{}", doodad.kind, doodad.id, doodad_ith).into()),
                    shadow_platform_ramp_mesh.clone(),
                    dooda_material,
                    shadow_platform_ramp_transform,
                ))
                .observe(update_info_on_click),
            _ => commands
                .spawn((
                    UnknownDoodadMaterial,
                    ObjectDoodadComponent,
                    DisplayInfoOnClick,
                    Name(format!("{}:{}:{}", doodad.kind, doodad.id, doodad_ith).into()),
                    unknown_object_mesh.clone(),
                    dooda_material,
                    shadow_platform_ramp_transform,
                ))
                .observe(update_info_on_click),
        };
    }
}
