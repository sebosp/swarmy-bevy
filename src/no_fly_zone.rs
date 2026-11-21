//! The no fly zones of the maps (the air units cannot enter them, i.e. the edges of the map): a
//! "no" sign on the ground, a red ring crossed by a bar as on a road sign, under a faint red
//! column of air with a few hoops up it, so that the zone reads as forbidden airspace. Shared
//! meshes (one set per radius) and materials, unlit.

use crate::MAP_SCALE_FACTOR;
use crate::cache_objects::ObjectPointComponent;
use bevy::prelude::*;
use std::collections::HashMap;

/// The height of the column of air, in cells, and the hoops up it.
pub const COLUMN_HEIGHT: f32 = 14.;
const HOOPS: [f32; 3] = [4.5, 9., 13.5];

/// The red of the sign, the color of a prohibition (with the shape of the sign, not alone).
const SIGN_COLOR: Color = Color::srgb(0.89, 0.29, 0.28);

/// The meshes of a zone of a radius.
#[derive(Clone)]
struct ZoneMeshes {
    column: Handle<Mesh>,
    ring: Handle<Mesh>,
    bar: Handle<Mesh>,
    hoop: Handle<Mesh>,
}

/// The shared meshes and materials of the no fly zones.
#[derive(Resource)]
pub struct NoFlyZoneAssets {
    sign: Handle<StandardMaterial>,
    column: Handle<StandardMaterial>,
    /// The hoops fade up the column.
    hoops: Vec<Handle<StandardMaterial>>,
    meshes: HashMap<u32, ZoneMeshes>,
}

pub struct NoFlyZonePlugin;

impl Plugin for NoFlyZonePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, make_assets);
    }
}

fn make_assets(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let translucent = |alpha: f32| StandardMaterial {
        base_color: SIGN_COLOR.with_alpha(alpha),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        // Seen from inside too.
        cull_mode: None,
        double_sided: true,
        ..default()
    };
    commands.insert_resource(NoFlyZoneAssets {
        sign: materials.add(StandardMaterial {
            base_color: SIGN_COLOR,
            // A little glow through the bloom, on the dark and the light ground.
            emissive: SIGN_COLOR.to_linear() * 0.8,
            unlit: true,
            ..default()
        }),
        column: materials.add(translucent(0.07)),
        hoops: HOOPS
            .iter()
            .enumerate()
            .map(|(hoop, _)| materials.add(translucent(0.55 - 0.15 * hoop as f32)))
            .collect(),
        meshes: HashMap::new(),
    });
}

impl NoFlyZoneAssets {
    fn meshes(&mut self, radius: f32, meshes: &mut Assets<Mesh>) -> ZoneMeshes {
        let r = radius * MAP_SCALE_FACTOR;
        self.meshes
            .entry((radius * 100.) as u32)
            .or_insert_with(|| {
                let tube = (radius * 0.08).clamp(0.12, 0.4) * MAP_SCALE_FACTOR;
                ZoneMeshes {
                    column: meshes.add(
                        Cylinder::new(r, COLUMN_HEIGHT * MAP_SCALE_FACTOR)
                            .mesh()
                            .resolution(32)
                            .build(),
                    ),
                    ring: meshes.add(Torus::new(r - 2. * tube, r)),
                    bar: meshes.add(Cuboid::new(2. * (r - tube), tube * 0.5, tube * 1.6)),
                    hoop: meshes.add(Torus::new(r - tube * 0.6, r)),
                }
            })
            .clone()
    }

    /// The column of a zone, its root: the entity clicked, centered at `COLUMN_HEIGHT / 2` above
    /// the ground. Its children are the sign on the ground and the hoops.
    pub fn zone(
        &mut self,
        radius: f32,
        meshes: &mut Assets<Mesh>,
    ) -> (
        (Mesh3d, MeshMaterial3d<StandardMaterial>),
        Vec<(
            ObjectPointComponent,
            Mesh3d,
            MeshMaterial3d<StandardMaterial>,
            Transform,
            Pickable,
        )>,
    ) {
        let zone = self.meshes(radius, meshes);
        let half = COLUMN_HEIGHT * 0.5 * MAP_SCALE_FACTOR;
        let ground = -half + 0.02 * MAP_SCALE_FACTOR;
        let child = |mesh: &Handle<Mesh>, material: &Handle<StandardMaterial>, transform| {
            (
                ObjectPointComponent,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                transform,
                Pickable::IGNORE,
            )
        };
        let mut children = vec![
            child(&zone.ring, &self.sign, Transform::from_xyz(0., ground, 0.)),
            // The bar from the upper left to the lower right as seen from the south, as on the
            // road signs.
            child(
                &zone.bar,
                &self.sign,
                Transform::from_xyz(0., ground, 0.)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)),
            ),
        ];
        for (height, material) in HOOPS.iter().zip(&self.hoops) {
            children.push(child(
                &zone.hoop,
                material,
                Transform::from_xyz(0., -half + height * MAP_SCALE_FACTOR, 0.),
            ));
        }
        (
            (Mesh3d(zone.column), MeshMaterial3d(self.column.clone())),
            children,
        )
    }
}
