//! The destructible rocks, debris and plates of the maps: a pile of rough boulders (rocks), of
//! broken metal chunks (debris) or a cover of hexagonal plates (unbuildable plates and bricks) on a
//! patch of rubble the size of their footprint, along the diagonal for the diagonal ones. A few
//! shared meshes (with a light baked in their vertex colors, see [`crate::baked_light`]) and
//! materials.

use crate::MAP_SCALE_FACTOR;
use crate::baked_light::{bake_flat_light, bake_smooth_light};
use crate::cache_objects::ObjectUnitComponent;
use crate::geyser_gas::noise;
use bevy::prelude::*;
use std::collections::HashMap;
use std::f32::consts::TAU;

/// The kinds of destructible objects, by their look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RockKind {
    Rock,
    Debris,
    Plates,
}

/// The shape of a destructible object read from its name: its kind, the side of its square
/// footprint in cells, and the direction of a diagonal one in the world (x, z).
#[derive(Debug, Clone, PartialEq)]
pub struct RockShape {
    pub kind: RockKind,
    pub size: f32,
    pub diagonal: Option<Vec2>,
}

/// Whether a unit of the map is a destructible rock, debris or plates.
pub fn is_destructible(name: &str) -> bool {
    name.starts_with("Destructible")
        || name.starts_with("Unbuildable")
        || name.contains("Debris")
        || name.contains("RockTowerDebris")
}

/// The shape of a destructible object from its name, i.e. `DestructibleRock6x6`,
/// `DestructibleRockEx1DiagonalHugeBLUR`, `UnbuildablePlatesDestructible`.
pub fn rock_shape(name: &str) -> RockShape {
    let kind = if name.contains("Plates") || name.contains("Bricks") {
        RockKind::Plates
    } else if name.contains("Debris") {
        RockKind::Debris
    } else {
        RockKind::Rock
    };
    let diagonal = name.contains("Diagonal").then(|| {
        // The map's x is the world's z, its y the world's x. BLUR goes from the bottom left to the
        // upper right of the map (x and y grow together), ULBR from the upper left to the bottom
        // right.
        if name.contains("ULBR") {
            Vec2::new(1., -1.).normalize()
        } else {
            Vec2::new(1., 1.).normalize()
        }
    });
    let size = if name.ends_with("6x6") {
        6.
    } else if name.ends_with("4x4") {
        4.
    } else if name.ends_with("2x2") || name.starts_with("Unbuildable") {
        2.
    } else if name.contains("Huge") {
        8.
    } else {
        4.
    };
    RockShape {
        kind,
        size,
        diagonal,
    }
}

/// A piece of a pile, in cells from the center of the object on the ground: which mesh, where,
/// its size and rotation.
#[derive(Debug, Clone, PartialEq)]
pub struct RockPiece {
    pub variant: usize,
    pub offset: Vec3,
    pub scale: Vec3,
    pub rotation: Quat,
}

/// The variants of the meshes of each kind.
const VARIANTS: usize = 3;

/// The pieces of an object, varied by a seed (its position): a big one in the middle and smaller
/// ones around it, or along the diagonal; the plates cover the footprint in a grid.
pub fn rock_pieces(shape: &RockShape, seed: f32) -> Vec<RockPiece> {
    let size = shape.size;
    let mut pieces = vec![];
    let mut piece = |index: usize, at: Vec2, radius: f32, flat: f32| {
        let s = seed + index as f32 * 5.17;
        let radius = radius * (0.85 + 0.3 * noise(s));
        pieces.push(RockPiece {
            variant: (noise(s + 1.) * VARIANTS as f32) as usize % VARIANTS,
            // Half in the ground.
            offset: Vec3::new(at.x, radius * flat * 0.25, at.y),
            scale: Vec3::new(radius, radius * flat, radius * (0.8 + 0.3 * noise(s + 2.))),
            rotation: Quat::from_rotation_y(noise(s + 3.) * TAU),
        });
    };
    match (shape.kind, shape.diagonal) {
        (RockKind::Plates, _) => {
            // Hexagonal plates of about a cell, tilted a little, side by side.
            let count = size.round().max(1.) as usize;
            let step = size / count as f32;
            for row in 0..count {
                for column in 0..count {
                    let index = row * count + column;
                    let s = seed + index as f32 * 3.3;
                    let at = Vec2::new(
                        (column as f32 + 0.5) * step - size / 2.,
                        (row as f32 + 0.5) * step - size / 2.,
                    );
                    pieces.push(RockPiece {
                        variant: index % VARIANTS,
                        offset: Vec3::new(at.x, 0.05, at.y),
                        scale: Vec3::new(step * 0.55, 0.12, step * 0.55),
                        rotation: Quat::from_rotation_y(noise(s) * TAU)
                            * Quat::from_rotation_x((noise(s + 1.) - 0.5) * 0.25),
                    });
                }
            }
        }
        (kind, Some(direction)) => {
            // Along the diagonal, the middle ones the biggest.
            let flat = if kind == RockKind::Debris { 0.6 } else { 0.75 };
            let length = size * std::f32::consts::SQRT_2 * 0.8;
            let count = (size * 1.2).round().max(3.) as usize;
            for index in 0..count {
                let t = index as f32 / (count - 1) as f32 - 0.5;
                let side =
                    Vec2::new(-direction.y, direction.x) * (noise(seed + index as f32) - 0.5);
                let at = direction * t * length + side * size * 0.2;
                piece(index, at, size * (0.22 - 0.12 * t.abs()), flat);
            }
        }
        (kind, None) => {
            let flat = if kind == RockKind::Debris { 0.6 } else { 0.75 };
            piece(0, Vec2::ZERO, size * 0.24, flat);
            let around = (size * 1.5).round().clamp(4., 9.) as usize;
            for index in 0..around {
                let angle =
                    (index as f32 + noise(seed + index as f32 * 1.9) * 0.6) * TAU / around as f32;
                let at = Vec2::from_angle(angle) * size * (0.3 + 0.06 * noise(seed + angle));
                piece(index + 1, at, size * 0.14, flat);
            }
        }
    }
    pieces
}

/// A boulder of radius 1: a coarse sphere with its points pushed in and out, flat shaded.
fn boulder_mesh(seed: f32) -> Mesh {
    let mut mesh = Sphere::new(1.)
        .mesh()
        .ico(1)
        .expect("a sphere of one subdivision");
    jitter(&mut mesh, seed, 0.72, 0.5);
    bake_flat_light(mesh)
}

/// A broken chunk of metal of side 1: a box with its corners pushed around.
fn debris_mesh(seed: f32) -> Mesh {
    let mut mesh = Cuboid::new(1.6, 1., 1.2).mesh().build();
    jitter(&mut mesh, seed, 0.6, 0.7);
    bake_flat_light(mesh)
}

/// A hexagonal plate of radius 1 and height 1.
fn plate_mesh() -> Mesh {
    bake_smooth_light(Cylinder::new(1., 1.).mesh().resolution(6).build())
}

/// Pushes the points of a mesh in and out by a noise of their position: the points shared by
/// several faces (the same position) move together.
fn jitter(mesh: &mut Mesh, seed: f32, from: f32, range: f32) {
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for position in positions.iter_mut() {
            let p = Vec3::from_array(*position);
            let bump = from + range * noise(seed + p.x * 31.7 + p.y * 17.3 + p.z * 11.1);
            *position = (p * bump).to_array();
        }
    }
}

/// The colors, unlit, multiplied by the light baked in the meshes.
const ROCK_COLOR: Color = Color::srgb(0.46, 0.42, 0.38);
const DEBRIS_COLOR: Color = Color::srgb(0.5, 0.36, 0.26);
const PLATE_COLOR: Color = Color::srgb(0.5, 0.54, 0.6);
const RUBBLE_COLOR: Color = Color::srgb(0.24, 0.22, 0.2);

/// The shared meshes and materials of the destructible objects.
#[derive(Resource)]
pub struct RockAssets {
    boulders: Vec<Handle<Mesh>>,
    debris: Vec<Handle<Mesh>>,
    plate: Handle<Mesh>,
    rock_material: Handle<StandardMaterial>,
    debris_material: Handle<StandardMaterial>,
    plate_material: Handle<StandardMaterial>,
    rubble_material: Handle<StandardMaterial>,
    /// The patches of rubble under the objects, one per size.
    rubble: HashMap<u32, Handle<Mesh>>,
}

pub struct RocksPlugin;

impl Plugin for RocksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, make_rock_assets);
    }
}

fn make_rock_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut unlit = |color: Color| {
        materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            ..default()
        })
    };
    commands.insert_resource(RockAssets {
        boulders: (0..VARIANTS)
            .map(|variant| meshes.add(boulder_mesh(variant as f32 * 11.3)))
            .collect(),
        debris: (0..VARIANTS)
            .map(|variant| meshes.add(debris_mesh(variant as f32 * 7.9)))
            .collect(),
        plate: meshes.add(plate_mesh()),
        rock_material: unlit(ROCK_COLOR),
        debris_material: unlit(DEBRIS_COLOR),
        plate_material: unlit(PLATE_COLOR),
        rubble_material: unlit(RUBBLE_COLOR),
        rubble: HashMap::new(),
    });
}

impl RockAssets {
    /// The patch of rubble of an object, on the ground at its center: it is the entity clicked.
    pub fn rubble(
        &mut self,
        shape: &RockShape,
        meshes: &mut Assets<Mesh>,
    ) -> (Mesh3d, MeshMaterial3d<StandardMaterial>) {
        // A diagonal object spreads along a longer line, its patch is wider.
        let radius = if shape.diagonal.is_some() {
            shape.size * 0.75
        } else {
            shape.size * 0.55
        };
        let key = (radius * 100.) as u32;
        let mesh = self
            .rubble
            .entry(key)
            .or_insert_with(|| {
                meshes.add(bake_smooth_light(
                    Cylinder::new(radius * MAP_SCALE_FACTOR, 0.05 * MAP_SCALE_FACTOR)
                        .mesh()
                        .resolution(10)
                        .build(),
                ))
            })
            .clone();
        (Mesh3d(mesh), MeshMaterial3d(self.rubble_material.clone()))
    }

    /// The pieces of an object, children of its patch of rubble at `origin`.
    pub fn pieces(
        &self,
        shape: &RockShape,
        origin: Vec3,
    ) -> Vec<(
        ObjectUnitComponent,
        Mesh3d,
        MeshMaterial3d<StandardMaterial>,
        Transform,
        Pickable,
    )> {
        let seed = origin.x * 12.9898 + origin.z * 78.233;
        rock_pieces(shape, seed)
            .into_iter()
            .map(|piece| {
                let (mesh, material) = match shape.kind {
                    RockKind::Rock => (&self.boulders[piece.variant], &self.rock_material),
                    RockKind::Debris => (&self.debris[piece.variant], &self.debris_material),
                    RockKind::Plates => (&self.plate, &self.plate_material),
                };
                (
                    ObjectUnitComponent,
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(piece.offset * MAP_SCALE_FACTOR)
                        .with_rotation(piece.rotation)
                        .with_scale(piece.scale * MAP_SCALE_FACTOR),
                    // The click goes to the patch, the object.
                    Pickable::IGNORE,
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_the_shape_of_the_rocks() {
        let rock = rock_shape("DestructibleRock6x6");
        assert_eq!(
            (rock.kind, rock.size, rock.diagonal),
            (RockKind::Rock, 6., None)
        );
        let huge = rock_shape("DestructibleRockEx1DiagonalHugeBLUR");
        assert_eq!(huge.size, 8.);
        let direction = huge.diagonal.unwrap();
        assert!(direction.x > 0. && direction.y > 0.);
        assert!(
            rock_shape("DestructibleDebrisRampDiagonalHugeULBR")
                .diagonal
                .unwrap()
                .y
                < 0.
        );
        assert_eq!(rock_shape("DestructibleDebris4x4").kind, RockKind::Debris);
        let plates = rock_shape("UnbuildablePlatesDestructible");
        assert_eq!((plates.kind, plates.size), (RockKind::Plates, 2.));
        assert!(is_destructible("UnbuildableRocksDestructible"));
        assert!(!is_destructible("MineralField"));
    }

    #[test]
    fn it_piles_the_pieces_within_the_footprint() {
        for name in [
            "DestructibleRock6x6",
            "DestructibleDebris4x4",
            "UnbuildablePlatesDestructible",
            "DestructibleRockEx1DiagonalHugeBLUR",
        ] {
            let shape = rock_shape(name);
            let pieces = rock_pieces(&shape, 1.5);
            assert!(pieces.len() >= 3, "{name}");
            // Within the footprint, or the length of the diagonal.
            let reach = shape.size * if shape.diagonal.is_some() { 0.75 } else { 0.55 };
            for piece in &pieces {
                assert!(
                    Vec2::new(piece.offset.x, piece.offset.z).length() <= reach + 0.01,
                    "{name}"
                );
                assert!(piece.variant < VARIANTS);
            }
        }
    }
}
