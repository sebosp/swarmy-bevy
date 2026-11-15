//! The gas of the geysers: puffs that rise from the green vent of each geyser, swell, drift with
//! the wind and fade, then start again. The puffs of all the geysers share one mesh and a few
//! materials (one per step of their fading), so that the window stays light.

use crate::MAP_SCALE_FACTOR;
use crate::cache_objects::{GeyserGas, ObjectUnitComponent};
use bevy::prelude::*;

/// The puffs of a geyser at a time.
const PUFFS_PER_GEYSER: usize = 7;

/// The seconds a puff lives, from the vent to gone.
const PUFF_LIFETIME: f32 = 3.5;

/// How high a puff rises in its life, and how big it gets, in map cells.
const PUFF_RISE: f32 = 6.0;
const PUFF_START_SIZE: f32 = 0.35;
const PUFF_END_SIZE: f32 = 1.6;

/// The steps of the fading of a puff, a material each.
const FADE_STEPS: usize = 8;

/// The direction the wind pushes the puffs, in map cells per second.
const WIND: Vec3 = Vec3::new(0.25, 0., 0.15);

/// A puff of gas, `age` from 0 to `PUFF_LIFETIME`, restarting at its vent.
#[derive(Component)]
pub struct GasPuff {
    vent: Vec3,
    age: f32,
    /// A small sideways drift of its own, so that the puffs do not stack.
    drift: Vec3,
}

/// The shared mesh and fading materials of the puffs.
#[derive(Resource)]
struct PuffAssets {
    mesh: Handle<Mesh>,
    fades: Vec<Handle<StandardMaterial>>,
}

pub struct GeyserGasPlugin;

impl Plugin for GeyserGasPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_puffs, move_puffs).chain());
    }
}

/// A pseudo random number from 0 to 1 for a seed, enough to vary the puffs.
pub fn noise(seed: f32) -> f32 {
    (seed.sin() * 43758.547).fract().abs()
}

/// Starts the puffs of the geysers once their vents are spawned.
fn spawn_puffs(
    mut commands: Commands,
    vents: Query<(&ChildOf, &Transform), Added<GeyserGas>>,
    geysers: Query<&Transform>,
    assets: Option<Res<PuffAssets>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if vents.is_empty() {
        return;
    }
    let assets = match assets {
        Some(assets) => (assets.mesh.clone(), assets.fades.clone()),
        None => {
            let mesh = meshes.add(Sphere::new(0.5).mesh().uv(12, 8));
            let fades: Vec<Handle<StandardMaterial>> = (0..FADE_STEPS)
                .map(|step| {
                    // Opaque-ish at the vent, gone at the top.
                    let alpha = 0.55 * (1. - step as f32 / FADE_STEPS as f32);
                    materials.add(StandardMaterial {
                        base_color: Color::srgba(0.45, 0.95, 0.5, alpha),
                        emissive: LinearRgba::rgb(0.15, 0.6, 0.2)
                            * (1. - step as f32 / FADE_STEPS as f32),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    })
                })
                .collect();
            commands.insert_resource(PuffAssets {
                mesh: mesh.clone(),
                fades: fades.clone(),
            });
            (mesh, fades)
        }
    };
    let (mesh, fades) = assets;
    for (child_of, vent) in &vents {
        let Ok(geyser) = geysers.get(child_of.parent()) else {
            continue;
        };
        let origin = geyser.translation + vent.translation;
        for puff in 0..PUFFS_PER_GEYSER {
            let seed = origin.x * 12.9898 + origin.z * 78.233 + puff as f32 * 3.7;
            commands.spawn((
                GasPuff {
                    vent: origin,
                    // Spread over the lifetime, so that the plume is continuous.
                    age: PUFF_LIFETIME * puff as f32 / PUFFS_PER_GEYSER as f32,
                    drift: Vec3::new(noise(seed) - 0.5, 0., noise(seed + 1.) - 0.5) * 0.6,
                },
                ObjectUnitComponent,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(fades[0].clone()),
                Transform::from_translation(origin).with_scale(Vec3::ZERO),
                Pickable::IGNORE,
            ));
        }
    }
}

/// Rises, swells, drifts and fades the puffs, a puff at the top starts again at its vent.
fn move_puffs(
    time: Res<Time>,
    assets: Option<Res<PuffAssets>>,
    mut puffs: Query<(
        &mut GasPuff,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let Some(assets) = assets else {
        return;
    };
    let dt = time.delta_secs();
    for (mut puff, mut transform, mut material) in &mut puffs {
        puff.age = (puff.age + dt) % PUFF_LIFETIME;
        let life = puff.age / PUFF_LIFETIME;
        // Fast out of the vent, slower as it spreads.
        let rise = PUFF_RISE * (1. - (1. - life).powi(2));
        let offset = Vec3::new(0., rise, 0.) + (WIND + puff.drift) * puff.age;
        transform.translation = puff.vent + offset * MAP_SCALE_FACTOR;
        let size = PUFF_START_SIZE + (PUFF_END_SIZE - PUFF_START_SIZE) * life;
        transform.scale = Vec3::splat(size * MAP_SCALE_FACTOR);
        let step = ((life * FADE_STEPS as f32) as usize).min(FADE_STEPS - 1);
        if material.0 != assets.fades[step] {
            material.0 = assets.fades[step].clone();
        }
    }
}
