//! The start locations as landing pads: a dark round platform the size of a town hall, a yellow
//! ring, a white H, and lights on its edge that run around it. All the pads share their meshes and
//! materials.

use crate::MAP_SCALE_FACTOR;
use crate::baked_light::bake_flat_light;
use crate::cache_objects::ObjectPointComponent;
use bevy::prelude::*;
use std::f32::consts::TAU;

/// The radius of a pad, in cells: a town hall covers 5x5 cells.
pub const PAD_RADIUS: f32 = 2.75;
/// The thickness of a pad, in cells.
pub const PAD_THICKNESS: f32 = 0.15;

/// The lights on the edge of a pad, and how many go around in a second.
const PAD_LIGHTS: usize = 12;
const LIGHTS_PER_SECOND: f32 = 8.;

/// The light of a pad, its place on the edge.
#[derive(Component)]
pub struct PadLight(usize);

/// The shared meshes and materials of the pads.
#[derive(Resource)]
pub struct HelipadAssets {
    pub pad_mesh: Handle<Mesh>,
    pub pad_material: Handle<StandardMaterial>,
    ring_mesh: Handle<Mesh>,
    ring_material: Handle<StandardMaterial>,
    leg_mesh: Handle<Mesh>,
    bar_mesh: Handle<Mesh>,
    marking_material: Handle<StandardMaterial>,
    light_mesh: Handle<Mesh>,
    light_on: Handle<StandardMaterial>,
    light_off: Handle<StandardMaterial>,
}

pub struct HelipadPlugin;

impl Plugin for HelipadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, make_helipad_assets)
            .add_systems(Update, run_pad_lights);
    }
}

fn make_helipad_assets(
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
    let assets = HelipadAssets {
        pad_material: unlit(Color::srgb(0.24, 0.25, 0.27)),
        ring_material: unlit(Color::srgb(0.95, 0.75, 0.1)),
        marking_material: unlit(Color::srgb(0.92, 0.92, 0.9)),
        light_on: unlit(LinearRgba::rgb(3.0, 2.0, 0.35).into()),
        light_off: unlit(Color::srgb(0.3, 0.25, 0.12)),
        pad_mesh: meshes.add(bake_flat_light(
            Cylinder::new(
                PAD_RADIUS * MAP_SCALE_FACTOR,
                PAD_THICKNESS * MAP_SCALE_FACTOR,
            )
            .mesh()
            .resolution(48)
            .build(),
        )),
        // A torus lies flat, its tube flattened by the transform of the ring.
        ring_mesh: meshes.add(Torus::new(
            (PAD_RADIUS - 0.75) * MAP_SCALE_FACTOR,
            (PAD_RADIUS - 0.45) * MAP_SCALE_FACTOR,
        )),
        leg_mesh: meshes.add(Cuboid::new(
            0.35 * MAP_SCALE_FACTOR,
            0.02 * MAP_SCALE_FACTOR,
            2.0 * MAP_SCALE_FACTOR,
        )),
        bar_mesh: meshes.add(Cuboid::new(
            1.1 * MAP_SCALE_FACTOR,
            0.02 * MAP_SCALE_FACTOR,
            0.35 * MAP_SCALE_FACTOR,
        )),
        light_mesh: meshes.add(Sphere::new(0.2 * MAP_SCALE_FACTOR)),
    };
    commands.insert_resource(assets);
}

/// Where the lights of a pad are, in cells from its center on its top.
pub fn light_positions() -> Vec<Vec3> {
    (0..PAD_LIGHTS)
        .map(|light| {
            let angle = light as f32 * TAU / PAD_LIGHTS as f32;
            let radius = PAD_RADIUS - 0.25;
            Vec3::new(
                radius * angle.cos(),
                PAD_THICKNESS * 0.5,
                radius * angle.sin(),
            )
        })
        .collect()
}

/// Draws the ring, the H and the lights on a pad, as its children.
pub fn add_pad_markings(pad: &mut EntityCommands, assets: &HelipadAssets) {
    let top = PAD_THICKNESS * 0.5 * MAP_SCALE_FACTOR;
    let marking = |mesh: &Handle<Mesh>, material: &Handle<StandardMaterial>, transform| {
        (
            ObjectPointComponent,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            transform,
        )
    };
    pad.with_children(|children| {
        children.spawn(marking(
            &assets.ring_mesh,
            &assets.ring_material,
            Transform::from_xyz(0., top, 0.).with_scale(Vec3::new(1., 0.02, 1.)),
        ));
        for side in [-1., 1.] {
            children.spawn(marking(
                &assets.leg_mesh,
                &assets.marking_material,
                Transform::from_xyz(side * 0.55 * MAP_SCALE_FACTOR, top, 0.),
            ));
        }
        children.spawn(marking(
            &assets.bar_mesh,
            &assets.marking_material,
            Transform::from_xyz(0., top, 0.),
        ));
        for (light, position) in light_positions().into_iter().enumerate() {
            children.spawn((
                marking(
                    &assets.light_mesh,
                    &assets.light_off,
                    Transform::from_translation(position * MAP_SCALE_FACTOR),
                ),
                PadLight(light),
            ));
        }
    });
}

/// Two lights run around each pad, on opposite sides.
fn run_pad_lights(
    time: Res<Time>,
    assets: Option<Res<HelipadAssets>>,
    mut lights: Query<(&PadLight, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let Some(assets) = assets else {
        return;
    };
    let step = (time.elapsed_secs() * LIGHTS_PER_SECOND) as usize;
    for (light, mut material) in &mut lights {
        let on = light.0 % (PAD_LIGHTS / 2) == step % (PAD_LIGHTS / 2);
        let wanted = if on {
            &assets.light_on
        } else {
            &assets.light_off
        };
        if material.0 != *wanted {
            material.0 = wanted.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_puts_the_lights_on_the_edge_of_the_pad() {
        let lights = light_positions();
        assert_eq!(lights.len(), PAD_LIGHTS);
        for light in lights {
            let radius = Vec2::new(light.x, light.z).length();
            assert!(radius < PAD_RADIUS && radius > PAD_RADIUS - 0.5);
            assert_eq!(light.y, PAD_THICKNESS * 0.5);
        }
    }
}
