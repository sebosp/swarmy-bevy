//! The mineral fields as ores: a lump of dark rock half in the ground, with glowing crystals
//! growing out of it, blue (gold for the rich fields). The crystals are pointed hexagonal shards,
//! each facet with its own brightness (see [`crate::baked_light`]), and they twinkle through the
//! camera's bloom. All the fields share a few meshes and materials.

use crate::MAP_SCALE_FACTOR;
use crate::baked_light::{AMBIENT, bake_flat_light};
use crate::cache_objects::ObjectUnitComponent;
use crate::geyser_gas::noise;
use bevy::prelude::*;
use std::f32::consts::TAU;

/// A mineral field covers 2x1 cells (2 along the map's x, the world's z), as in the game. In
/// cells: the lump of rock, half of it under the ground.
pub const ORE_SIZE: [f32; 3] = [1.1, 0.4, 2.1];

/// The crystals of a field.
const SHARDS_PER_FIELD: usize = 7;

/// The sides of a shard, and where its point starts, from 0 at its base to 1 at the point.
const SHARD_SIDES: usize = 6;
const SHARD_SHOULDER: f32 = 0.72;

/// The materials of each kind of crystal, each twinkling at its own time.
const SHINE_PHASES: usize = 3;

/// The glow of the crystals, linear, multiplied by the brightness of each facet: above 1 the
/// camera's bloom makes them shine.
const MINERAL_GLOW: LinearRgba = LinearRgba::rgb(0.08, 0.45, 1.7);
const RICH_MINERAL_GLOW: LinearRgba = LinearRgba::rgb(1.5, 0.95, 0.16);

/// The rock of the ore, when the terrain is not painted with its textures.
const ORE_COLOR: Color = Color::srgb(0.32, 0.29, 0.27);

/// The kinds of mineral fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Crystal {
    Blue,
    Gold,
}

/// The shared meshes and materials of the mineral fields.
#[derive(Resource)]
pub struct MineralAssets {
    pub ore_mesh: Handle<Mesh>,
    pub ore_material: Handle<StandardMaterial>,
    shard_mesh: Handle<Mesh>,
    blue: Vec<Handle<StandardMaterial>>,
    gold: Vec<Handle<StandardMaterial>>,
}

pub struct MineralsPlugin;

impl Plugin for MineralsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, make_mineral_assets)
            .add_systems(Update, shine_crystals);
    }
}

/// A crystal of a field, in cells from the center of its field: where its base is, its size
/// (radius, height, radius) and its lean.
#[derive(Debug, Clone, PartialEq)]
pub struct ShardPlacement {
    pub offset: Vec3,
    pub scale: Vec3,
    pub rotation: Quat,
}

/// The crystals of a field, from a seed that varies them from a field to the next: spread along
/// the field, the middle ones the tallest, leaning out of it.
pub fn shard_layout(seed: f32) -> Vec<ShardPlacement> {
    (0..SHARDS_PER_FIELD)
        .map(|shard| {
            let s = seed + shard as f32 * 7.31;
            let along = shard as f32 / (SHARDS_PER_FIELD - 1) as f32 - 0.5;
            let offset = Vec3::new(
                (noise(s) - 0.5) * 0.6,
                0.,
                along * 1.5 + (noise(s + 1.) - 0.5) * 0.2,
            );
            let middle = 1. - along.abs() * 1.2;
            let height = 0.5 + 0.55 * middle + 0.35 * noise(s + 2.);
            let radius = 0.13 + 0.09 * noise(s + 3.);
            let lean = Vec3::new(offset.x * 0.9, 1., offset.z * 0.45).normalize();
            let rotation =
                Quat::from_rotation_arc(Vec3::Y, lean) * Quat::from_rotation_y(noise(s + 4.) * TAU);
            ShardPlacement {
                offset,
                scale: Vec3::new(radius, height, radius),
                rotation,
            }
        })
        .collect()
}

/// A crystal of radius and height 1: a hexagonal prism, a little narrower at its base, ending in a
/// point. Its base is open, it is in the rock.
pub fn shard_mesh() -> Mesh {
    let ring = |y: f32, radius: f32| -> Vec<[f32; 3]> {
        (0..SHARD_SIDES)
            .map(|side| {
                let angle = side as f32 * TAU / SHARD_SIDES as f32;
                [radius * angle.cos(), y, radius * angle.sin()]
            })
            .collect()
    };
    let mut positions = ring(0., 0.8);
    positions.extend(ring(SHARD_SHOULDER, 1.));
    positions.push([0., 1., 0.]);
    let tip = (2 * SHARD_SIDES) as u32;
    let mut indices = Vec::new();
    for side in 0..SHARD_SIDES as u32 {
        let next = (side + 1) % SHARD_SIDES as u32;
        let (base, base_next) = (side, next);
        let (shoulder, shoulder_next) = (side + SHARD_SIDES as u32, next + SHARD_SIDES as u32);
        // Counterclockwise seen from outside.
        indices.extend([base, shoulder_next, base_next]);
        indices.extend([base, shoulder, shoulder_next]);
        indices.extend([shoulder, tip, shoulder_next]);
    }
    let count = positions.len();
    let mut mesh = bake_flat_light(
        Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; count])
        .with_inserted_indices(bevy::mesh::Indices::U32(indices)),
    );
    if let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
    {
        for color in colors.iter_mut() {
            let light = crystal_light(color[0]);
            *color = [light, light, light, 1.];
        }
    }
    mesh
}

/// The facets of a crystal contrast more than a rock's: the ones facing the light shine above 1
/// (the bloom catches them), the others are dark. From the baked brightness of a facet.
pub fn crystal_light(baked: f32) -> f32 {
    let facing = ((baked - AMBIENT) / (1. - AMBIENT)).clamp(0., 1.);
    0.3 + 1.2 * facing * facing
}

/// The lump of rock: a coarse sphere, its points pushed in and out so that it is not round, then
/// stretched to the field.
fn ore_mesh() -> Mesh {
    let mut mesh = Sphere::new(0.5)
        .mesh()
        .ico(1)
        .expect("a sphere of one subdivision");
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for position in positions.iter_mut() {
            let p = Vec3::from_array(*position);
            let bump = 0.8 + 0.4 * noise(p.x * 31.7 + p.y * 17.3 + p.z * 11.1);
            *position = (p * bump).to_array();
        }
    }
    bake_flat_light(mesh.scaled_by(Vec3::from_array(ORE_SIZE) * MAP_SCALE_FACTOR))
}

fn make_mineral_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut crystals = |glow: LinearRgba| {
        (0..SHINE_PHASES)
            .map(|_| {
                materials.add(StandardMaterial {
                    base_color: glow.into(),
                    unlit: true,
                    ..default()
                })
            })
            .collect::<Vec<_>>()
    };
    let blue = crystals(MINERAL_GLOW);
    let gold = crystals(RICH_MINERAL_GLOW);
    commands.insert_resource(MineralAssets {
        ore_mesh: meshes.add(ore_mesh()),
        ore_material: materials.add(StandardMaterial {
            base_color: ORE_COLOR,
            unlit: true,
            ..default()
        }),
        shard_mesh: meshes.add(shard_mesh()),
        blue,
        gold,
    });
}

/// The crystals of a field, children of its ore (the entity at `origin`, on the ground).
pub fn crystals_of_field(
    assets: &MineralAssets,
    crystal: Crystal,
    origin: Vec3,
) -> Vec<(
    ObjectUnitComponent,
    Mesh3d,
    MeshMaterial3d<StandardMaterial>,
    Transform,
)> {
    let materials = match crystal {
        Crystal::Blue => &assets.blue,
        Crystal::Gold => &assets.gold,
    };
    let seed = origin.x * 12.9898 + origin.z * 78.233;
    shard_layout(seed)
        .into_iter()
        .enumerate()
        .map(|(shard, placed)| {
            (
                ObjectUnitComponent,
                Mesh3d(assets.shard_mesh.clone()),
                MeshMaterial3d(materials[shard % SHINE_PHASES].clone()),
                Transform::from_translation(placed.offset * MAP_SCALE_FACTOR)
                    .with_rotation(placed.rotation)
                    .with_scale(placed.scale * MAP_SCALE_FACTOR),
            )
        })
        .collect()
}

/// The crystals twinkle: their glow rises in short sparks, each material at its own time.
fn shine_crystals(
    time: Res<Time>,
    assets: Option<Res<MineralAssets>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(assets) = assets else {
        return;
    };
    let t = time.elapsed_secs();
    for (handles, glow) in [
        (&assets.blue, MINERAL_GLOW),
        (&assets.gold, RICH_MINERAL_GLOW),
    ] {
        for (phase, handle) in handles.iter().enumerate() {
            // A slow breath and a sharp spark now and then.
            let wave = 0.5 + 0.5 * (t * 1.3 + phase as f32 * 2.1).sin();
            let spark = (0.5 + 0.5 * (t * 0.7 + phase as f32 * 1.7).sin()).powi(12);
            let strength = 0.8 + 0.3 * wave + 1.2 * spark;
            if let Some(mut material) = materials.get_mut(handle) {
                material.base_color = (glow * strength).into();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;

    #[test]
    fn it_turns_the_facets_of_a_shard_outward() {
        let mesh = shard_mesh();
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("no positions");
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("no normals");
        };
        assert_eq!(positions.len(), SHARD_SIDES * 9);
        for (triangle, normals) in positions.chunks(3).zip(normals.chunks(3)) {
            let center = triangle.iter().map(|p| Vec3::from_array(*p)).sum::<Vec3>() / 3.;
            let outward = Vec3::new(center.x, 0., center.z);
            assert!(
                Vec3::from_array(normals[0]).dot(outward) > 0.,
                "{triangle:?}"
            );
        }
    }

    #[test]
    fn it_makes_the_facets_of_the_crystals_contrast() {
        assert_eq!(crystal_light(AMBIENT), 0.3);
        assert_eq!(crystal_light(1.), 1.5);
    }

    #[test]
    fn it_grows_the_crystals_on_the_field() {
        let shards = shard_layout(42.);
        assert_eq!(shards.len(), SHARDS_PER_FIELD);
        for shard in &shards {
            assert!(shard.offset.x.abs() <= ORE_SIZE[0] / 2.);
            assert!(shard.offset.z.abs() <= ORE_SIZE[2] / 2.);
            assert!(shard.scale.y > 0.4 && shard.scale.y < 1.5);
            // Leaning, but up.
            assert!((shard.rotation * Vec3::Y).y > 0.8);
        }
        // Each field its own.
        assert_ne!(shards, shard_layout(43.));
    }
}
