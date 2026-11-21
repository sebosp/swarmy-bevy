//! Loads the Object file contained in the MPQ.

use super::t3_height_map::T3HeightMapResource;
use crate::MAP_SCALE_FACTOR;
use crate::MapScene;
use crate::helipad::{HelipadAssets, PAD_THICKNESS, add_pad_markings};
use crate::minerals::{Crystal, MineralAssets, crystals_of_field};
use crate::no_fly_zone::{COLUMN_HEIGHT, NoFlyZoneAssets};
use crate::rocks::{RockAssets, is_destructible, rock_shape};
use crate::standard_material_from_gltf_material;
use crate::swarmy_feathers::{DisplayInfoOnClick, update_info_on_click};
use crate::t3_height_map::{CELL_HEIGHT_MULTIPLIER, terrain_top};
use bevy::gltf::GltfMaterial;
use bevy::prelude::*;

pub mod doodas;

// A few Object Units.
pub const XEL_NAGA_TOWER_HEIGHT: f32 = 25.0;
pub const XEL_NAGA_TOWER_RADIUS: f32 = 0.5;

pub const UNKNOWN_OBJECT_SIZE: f32 = 1.0;

/// A geyser covers 3x3 cells, as in the game, in cells. The mineral fields are in
/// [`crate::minerals`].
pub const GEYSER_SIZE: [f32; 3] = [3.0, 0.6, 3.0];

/// The rock painted with a texture when the terrain is (see [`crate::terrain_textures`]): the
/// geysers and the ore of the mineral fields.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RockTexture;

/// The gas of a geyser: a glowing vent in the middle of its ring, green, violet for a rich geyser
/// (see [`crate::geyser_gas`]).
#[derive(Component, Default)]
pub struct GeyserGas {
    pub rich: bool,
}

// Testing Object Points.
/// The radius of a no fly zone drawn, by its soft pathing radius.
pub const NO_FLY_ZONE_RADIUS: f32 = 0.5;

/// A resources that contains the Objects XML file contents mirror copy for Bevy.
#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default)]
pub struct PlacedObjectsResource {
    pub version: u32,
    pub points: Vec<ObjectPoint>,
    pub doodas: Vec<ObjectDoodad>,
    pub units: Vec<ObjectUnit>,
}

impl From<s2protocol::cache_handles::cache_objects::PlacedObjects> for PlacedObjectsResource {
    fn from(src: s2protocol::cache_handles::cache_objects::PlacedObjects) -> PlacedObjectsResource {
        PlacedObjectsResource {
            version: src.version,
            points: src.points.into_iter().map(|x| x.into()).collect(),
            doodas: src.doodas.into_iter().map(|x| x.into()).collect(),
            units: src.units.into_iter().map(|x| x.into()).collect(),
        }
    }
}

/// A copy of the Doodad Object for Reflect, etc.
#[derive(Default, Reflect, Debug)]
#[reflect(Default)]
pub struct ObjectDoodad {
    pub id: String,
    pub variation: String,
    pub position: Option<String>,
    pub rotation: String,
    pub scale: String,
    pub kind: String,
}

impl From<s2protocol::cache_handles::cache_objects::ObjectDoodad> for ObjectDoodad {
    fn from(src: s2protocol::cache_handles::cache_objects::ObjectDoodad) -> ObjectDoodad {
        ObjectDoodad {
            id: src.id,
            variation: src.variation,
            position: src.position,
            rotation: src.rotation,
            scale: src.scale,
            kind: src.kind,
        }
    }
}

#[derive(Default, Reflect, Debug)]
#[reflect(Default)]
pub struct ObjectPoint {
    pub id: String,
    pub position: Option<String>,
    pub scale: String,
    pub kind: String,
    pub name: String,
    pub color: Option<String>,
    pub pathing_radius_soft: f32,
    pub pathing_radius_hard: f32,
}

impl From<s2protocol::cache_handles::cache_objects::ObjectPoint> for ObjectPoint {
    fn from(src: s2protocol::cache_handles::cache_objects::ObjectPoint) -> ObjectPoint {
        ObjectPoint {
            id: src.id,
            position: src.position,
            scale: src.scale,
            kind: src.kind,
            name: src.name,
            color: src.color,
            pathing_radius_soft: src.pathing_radius_soft,
            pathing_radius_hard: src.pathing_radius_hard,
        }
    }
}

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
pub struct ObjectUnit {
    pub id: String,
    pub variation: String,
    pub position: Option<String>,
    pub scale: String,
    pub unit_kind: String,
}

impl From<s2protocol::cache_handles::cache_objects::ObjectUnit> for ObjectUnit {
    fn from(src: s2protocol::cache_handles::cache_objects::ObjectUnit) -> ObjectUnit {
        ObjectUnit {
            id: src.id,
            variation: src.variation,
            position: src.position,
            scale: src.scale,
            unit_kind: src.unit_kind,
        }
    }
}

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct RichMineralField750Material;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct RichMineralFieldMaterial;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct MineralField750Material;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct MineralFieldMaterial;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct VespeneGeyserMaterial;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct RichVespeneGeyserMaterial;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct SpacePlatformGeyserMaterial;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct XelNagaTowerMaterial;

/// A destructible rock, debris or plates, see [`crate::rocks`].
#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct DestructibleRockMaterial;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct UnknownUnit;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct NoFlyZoneMaterial;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct UnknownObjectPointMaterial;

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
pub struct StartLocMaterial;

/// Marks the ObjectUnits, mineral fields, geysers, rocks, etc, to show or hide them from the
/// View menu.
#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
pub struct ObjectUnitComponent;

/// Marks the ObjectPoints, start locations, no fly zones, etc, to show or hide them from the
/// View menu.
#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
pub struct ObjectPointComponent;

/// Loads the t3 height map.
pub fn load_cache_objects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    t3_height_map_res: Res<T3HeightMapResource>,
    map_scene: Res<MapScene>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_materials: Res<Assets<GltfMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    placed_objects: Res<PlacedObjectsResource>,
    minerals: Res<MineralAssets>,
    helipads: Res<HelipadAssets>,
    mut rocks: ResMut<RockAssets>,
    mut no_fly_zones: ResMut<NoFlyZoneAssets>,
    mut loaded: Local<bool>,
) -> Result<(), BevyError> {
    // Only do this once
    if *loaded {
        return Ok(());
    }
    // Wait until the scene and its materials are loaded, the objects are only spawned once.
    let Some(gltf) = gltf_assets.get(&map_scene.0) else {
        return Ok(());
    };
    if !gltf
        .named_materials
        .values()
        .all(|material| gltf_materials.contains(material))
    {
        return Ok(());
    }

    let mineral_mesh = Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(
        MAP_SCALE_FACTOR,
        MAP_SCALE_FACTOR,
        MAP_SCALE_FACTOR,
    ))));
    // A geyser is a ring of rock around its glowing vent (the gas rises from it), no block.
    let geyser_mesh = Mesh3d(
        meshes.add(crate::baked_light::bake_smooth_light(
            Torus::new(
                GEYSER_SIZE[0] * 0.3 * MAP_SCALE_FACTOR,
                GEYSER_SIZE[0] * 0.5 * MAP_SCALE_FACTOR,
            )
            .mesh()
            .build()
            .scaled_by(Vec3::new(1., GEYSER_SIZE[1] / GEYSER_SIZE[0] * 3., 1.)),
        )),
    );
    let geyser_material = MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Color::srgb(0.36, 0.33, 0.3),
        unlit: true,
        ..default()
    }));
    let gas_mesh = Mesh3d(meshes.add(Cylinder::new(
        GEYSER_SIZE[0] * 0.3 * MAP_SCALE_FACTOR,
        0.04 * MAP_SCALE_FACTOR,
    )));
    let gas_material = MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.9, 0.35),
        emissive: LinearRgba::rgb(0.3, 2.0, 0.5),
        ..default()
    }));
    let rich_gas_material = MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.35, 0.95),
        emissive: LinearRgba::rgb(1.2, 0.35, 2.0),
        ..default()
    }));
    let xel_naga_mesh = Mesh3d(meshes.add(Cylinder::new(
        XEL_NAGA_TOWER_RADIUS * MAP_SCALE_FACTOR,
        XEL_NAGA_TOWER_HEIGHT * MAP_SCALE_FACTOR,
    )));
    for unit in &placed_objects.units {
        tracing::info!("{:?}", unit);
        // ObjectUnit { id: "209", variation: "8", position: "97,102.5,0", scale: "1,1,1", unit_kind: "RichMineralField" }
        let unit_pos: Vec<f32> = match unit.position {
            Some(ref val) => val
                .split(",")
                .filter_map(|x| x.parse::<f32>().ok())
                .collect(),
            None => continue,
        };
        if unit_pos.len() != 3 {
            tracing::error!(
                "Unexpected number of tokens for unit position typed: {}",
                unit.unit_kind
            );
            continue;
        }
        if is_destructible(&unit.unit_kind) {
            // A pile of rocks, debris or plates on a patch of rubble, see [`crate::rocks`].
            let (x, y) = (unit_pos[0], unit_pos[1]);
            let cell_height = t3_height_map_res
                .data
                .get(y as usize * t3_height_map_res.width + x as usize)
                .copied()
                .unwrap_or_default();
            let shape = rock_shape(&unit.unit_kind);
            let origin = Vec3::new(
                MAP_SCALE_FACTOR * y,
                terrain_top(cell_height as f32),
                MAP_SCALE_FACTOR * x,
            );
            commands
                .spawn((
                    DestructibleRockMaterial,
                    rocks.rubble(&shape, &mut meshes),
                    Transform::from_translation(origin),
                    Children::spawn(SpawnIter(rocks.pieces(&shape, origin).into_iter())),
                    ObjectUnitComponent,
                    DisplayInfoOnClick,
                    Name(format!("{}:{}", unit.unit_kind, unit.id).into()),
                ))
                .observe(update_info_on_click);
            continue;
        }
        let Some(unit_handle) = gltf.named_materials.get(unit.unit_kind.as_str()) else {
            tracing::warn!(
                "Unhandled Skein GLTF named_material Unit: {}",
                unit.unit_kind
            );
            continue;
        };
        let Some(unit_gltf_material) = gltf_materials.get(unit_handle.id()) else {
            tracing::warn!(
                "Unhandled Skein GLTF gltf_material Unit: {}",
                unit.unit_kind
            );
            continue;
        };
        let unit_material =
            MeshMaterial3d(materials.add(standard_material_from_gltf_material(unit_gltf_material)));
        let x = unit_pos[0];
        let y = unit_pos[1];
        let target_vec_pos = y as usize * t3_height_map_res.width + x as usize;
        let cell_height = t3_height_map_res.data[target_vec_pos];
        let mineral_transform = Transform::from_xyz(
            MAP_SCALE_FACTOR * y,
            cell_height as f32 * CELL_HEIGHT_MULTIPLIER * MAP_SCALE_FACTOR,
            MAP_SCALE_FACTOR * x,
        );
        // The mineral fields and geysers stand on the terrain.
        let on_terrain = |height: f32| {
            Transform::from_xyz(
                MAP_SCALE_FACTOR * y,
                terrain_top(cell_height as f32) + height * 0.5 * MAP_SCALE_FACTOR,
                MAP_SCALE_FACTOR * x,
            )
        };
        // The ore of a mineral field is half in the ground, its crystals grow out of it.
        let field_transform = on_terrain(0.);
        let field = |crystal: Crystal| {
            (
                RockTexture,
                Mesh3d(minerals.ore_mesh.clone()),
                MeshMaterial3d(minerals.ore_material.clone()),
                field_transform,
                Children::spawn(SpawnIter(
                    crystals_of_field(&minerals, crystal, field_transform.translation).into_iter(),
                )),
            )
        };
        let geyser_transform = on_terrain(0.);
        let xel_naga_transform = Transform::from_xyz(
            MAP_SCALE_FACTOR * y,
            XEL_NAGA_TOWER_HEIGHT * 0.5 * MAP_SCALE_FACTOR,
            MAP_SCALE_FACTOR * x,
        );
        // The vent of a geyser, in the middle of its ring, a little above the ground.
        let geyser = |material: MeshMaterial3d<StandardMaterial>, rich: bool| {
            (
                RockTexture,
                geyser_mesh.clone(),
                geyser_material.clone(),
                geyser_transform,
                children![(
                    GeyserGas { rich },
                    ObjectUnitComponent,
                    gas_mesh.clone(),
                    material,
                    Transform::from_xyz(0., 0.05 * MAP_SCALE_FACTOR, 0.),
                    Pickable::IGNORE,
                )],
            )
        };
        let mut spawned = match unit.unit_kind.as_ref() {
            "RichMineralField750" => {
                commands.spawn((RichMineralField750Material, field(Crystal::Gold)))
            }
            "RichMineralField" => commands.spawn((RichMineralFieldMaterial, field(Crystal::Gold))),
            "MineralField750" => commands.spawn((MineralField750Material, field(Crystal::Blue))),
            "MineralField" => commands.spawn((MineralFieldMaterial, field(Crystal::Blue))),
            "RichVespeneGeyser" => commands.spawn((
                RichVespeneGeyserMaterial,
                geyser(rich_gas_material.clone(), true),
            )),
            "VespeneGeyser" => {
                commands.spawn((VespeneGeyserMaterial, geyser(gas_material.clone(), false)))
            }
            "SpacePlatformGeyser" => commands.spawn((
                SpacePlatformGeyserMaterial,
                geyser(gas_material.clone(), false),
            )),
            "XelNagaTower" => commands.spawn((
                XelNagaTowerMaterial,
                xel_naga_mesh.clone(),
                unit_material,
                xel_naga_transform,
            )),
            _ => {
                tracing::warn!("Unhandled unit_kind: {}", unit.unit_kind);
                commands.spawn((
                    RichMineralFieldMaterial,
                    mineral_mesh.clone(),
                    unit_material,
                    mineral_transform,
                ))
            }
        };
        spawned
            .insert((
                ObjectUnitComponent,
                DisplayInfoOnClick,
                Name(format!("{}:{}", unit.unit_kind, unit.id).into()),
            ))
            .observe(update_info_on_click);
    }
    let unknown_object_mesh = Mesh3d(meshes.add(Cuboid::new(
        UNKNOWN_OBJECT_SIZE * MAP_SCALE_FACTOR,
        UNKNOWN_OBJECT_SIZE * MAP_SCALE_FACTOR,
        UNKNOWN_OBJECT_SIZE * MAP_SCALE_FACTOR,
    )));
    // Id="1035" Position="6.0996,150.3146,0" Scale="1,1,1" Type="NoFlyZone" Name="No Fly Zone 011" Color="0,0,0,0" PathingRadiusSoft="5" PathingRadiusHard="4"
    for object_point in &placed_objects.points {
        // These are objects in the map, decorations, animation references, etc.
        let unit_pos: Vec<f32> = match object_point.position {
            Some(ref val) => val
                .split(",")
                .filter_map(|x| x.parse::<f32>().ok())
                .collect(),
            None => continue,
        };
        if unit_pos.len() != 3 {
            tracing::error!(
                "Unexpected number of tokens for unit position typed: {}",
                object_point.kind
            );
            continue;
        }
        let x = unit_pos[0];
        let y = unit_pos[1];
        let z = unit_pos[2];
        let target_vec_pos = y as usize * t3_height_map_res.width + x as usize;
        let cell_height = t3_height_map_res.data[target_vec_pos];

        let Some(unit_handle) = gltf.named_materials.get(object_point.kind.as_str()) else {
            tracing::warn!(
                "Unhandled Skein GLTF named_material ObjectPoint: {}",
                object_point.kind
            );
            continue;
        };
        let Some(unit_gltf_material) = gltf_materials.get(unit_handle.id()) else {
            tracing::warn!(
                "Unhandled Skein GLTF gltf_material ObjectPoint: {}",
                object_point.kind
            );
            continue;
        };
        let object_point_material =
            MeshMaterial3d(materials.add(standard_material_from_gltf_material(unit_gltf_material)));
        let pathing_radius_soft = object_point.pathing_radius_soft;
        let _pathing_radius_hard = object_point.pathing_radius_hard;
        let cylinder_transform = Transform::from_xyz(
            MAP_SCALE_FACTOR * y,
            z + cell_height as f32 * CELL_HEIGHT_MULTIPLIER * MAP_SCALE_FACTOR,
            MAP_SCALE_FACTOR * x,
        );
        // TODO: We should use the alpha channel, dunno if we need glsl for that tho because it's
        // being set from blender and exported to gltf.
        let mut spawned = match object_point.kind.as_str() {
            "NoFlyZone" => {
                // A "no" sign on the ground under a faint column of air, see
                // [`crate::no_fly_zone`].
                let (column, children) =
                    no_fly_zones.zone(pathing_radius_soft * NO_FLY_ZONE_RADIUS, &mut meshes);
                commands.spawn((
                    NoFlyZoneMaterial,
                    column,
                    Transform::from_xyz(
                        MAP_SCALE_FACTOR * y,
                        terrain_top(cell_height as f32) + COLUMN_HEIGHT * 0.5 * MAP_SCALE_FACTOR,
                        MAP_SCALE_FACTOR * x,
                    ),
                    Children::spawn(SpawnIter(children.into_iter())),
                ))
            }
            "StartLoc" => {
                // A landing pad on the ground, see [`crate::helipad`].
                let mut pad = commands.spawn((
                    StartLocMaterial,
                    Mesh3d(helipads.pad_mesh.clone()),
                    MeshMaterial3d(helipads.pad_material.clone()),
                    Transform::from_xyz(
                        MAP_SCALE_FACTOR * y,
                        terrain_top(cell_height as f32) + PAD_THICKNESS * 0.5 * MAP_SCALE_FACTOR,
                        MAP_SCALE_FACTOR * x,
                    ),
                ));
                add_pad_markings(&mut pad, &helipads);
                pad
            }
            _ => commands.spawn((
                UnknownObjectPointMaterial,
                unknown_object_mesh.clone(),
                object_point_material,
                cylinder_transform,
            )),
        };
        spawned
            .insert((
                ObjectPointComponent,
                DisplayInfoOnClick,
                Name(format!("{}:{}", object_point.kind, object_point.name).into()),
            ))
            .observe(update_info_on_click);
    }
    *loaded = true;
    Ok(())
}
