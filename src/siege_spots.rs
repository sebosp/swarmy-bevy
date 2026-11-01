//! Draws where the Siege Tanks sieged on the map and the enemy units they killed there, from the
//! games of a swarmy snapshot: a bar per spot on top of the terrain, its height is the kills and
//! its color goes from yellow to red with the kills per siege. Clicking a bar shows its numbers.
//!
//! swarmy writes the spots as JSON, see `SiegeSpots` in swarmy-common, and passes the file with
//! `--siege-spots`.

use crate::MAP_SCALE_FACTOR;
use crate::swarmy_feathers::{DisplayInfoOnClick, update_info_on_click};
use crate::t3_height_map::{T3HeightMapResource, terrain_top};
use bevy::color::palettes;
use bevy::prelude::*;
use serde::Deserialize;

/// The tallest bar, for the spot with the most kills, in world units.
const MAX_BAR_HEIGHT: f32 = 4.;

/// The bars cover this share of their spot, so that the spots next to each other are apart.
const BAR_SHARE: f32 = 0.8;

/// A spot of the map where tanks sieged.
#[derive(Debug, Clone, Deserialize)]
pub struct SiegeSpot {
    /// The spot, the map cell divided by [`SiegeSpots::spot_cells`].
    pub x: u32,
    pub y: u32,
    pub sieges: usize,
    pub kills: usize,
    pub games: usize,
}

impl SiegeSpot {
    pub fn kills_per_siege(&self) -> f32 {
        self.kills as f32 / self.sieges.max(1) as f32
    }
}

/// The siege spots of a map, the most kills first.
#[derive(Debug, Clone, Deserialize, Resource)]
pub struct SiegeSpots {
    pub map_title: String,
    pub spot_cells: u32,
    pub games: usize,
    pub sieges: usize,
    pub kills: usize,
    pub spots: Vec<SiegeSpot>,
}

impl SiegeSpots {
    pub fn load(path: &str) -> Result<Self, String> {
        let json = std::fs::read_to_string(path).map_err(|err| format!("{path}: {err}"))?;
        serde_json::from_str(&json).map_err(|err| format!("{path}: {err}"))
    }

    /// The center of a spot in map cells.
    fn center(&self, spot: &SiegeSpot) -> (f32, f32) {
        let size = self.spot_cells as f32;
        (
            spot.x as f32 * size + size / 2.,
            spot.y as f32 * size + size / 2.,
        )
    }
}

/// Yellow for the fewest kills per siege, red for the most.
fn spot_color(kills_per_siege: f32, max_kills_per_siege: f32) -> Color {
    let share = (kills_per_siege / max_kills_per_siege.max(1.)).clamp(0., 1.);
    Color::from(palettes::tailwind::YELLOW_400)
        .mix(&Color::from(palettes::tailwind::RED_600), share)
}

pub struct SiegeSpotsPlugin;

impl Plugin for SiegeSpotsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_legend).add_systems(
            Update,
            spawn_spots.run_if(resource_exists::<T3HeightMapResource>),
        );
    }
}

/// Spawns the bars once the height map is loaded, the bars stand on the terrain of their spot.
fn spawn_spots(
    mut commands: Commands,
    spots: Res<SiegeSpots>,
    height_map: Res<T3HeightMapResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawned: Local<bool>,
) {
    if *spawned {
        return;
    }
    *spawned = true;
    let max_kills = spots
        .spots
        .iter()
        .map(|spot| spot.kills)
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let max_kills_per_siege = spots
        .spots
        .iter()
        .map(SiegeSpot::kills_per_siege)
        .fold(0., f32::max);
    let side = spots.spot_cells as f32 * BAR_SHARE * MAP_SCALE_FACTOR;
    let mesh = Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(side, 1., side))));
    for (rank, spot) in spots.spots.iter().enumerate() {
        let (map_x, map_y) = spots.center(spot);
        let ground = height_map
            .height_at(map_x as i32, map_y as i32)
            .map(|height| terrain_top(height as f32))
            .unwrap_or_default();
        // The spots without kills are a thin plate, so that they are seen.
        let height = (spot.kills as f32 / max_kills * MAX_BAR_HEIGHT).max(0.05);
        let color = spot_color(spot.kills_per_siege(), max_kills_per_siege);
        let material = MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: color.to_linear() * 0.6,
            ..default()
        }));
        let cells = spots.spot_cells;
        let (x, y) = (spot.x * cells, spot.y * cells);
        commands
            .spawn((
                Name::new(format!(
                    "Siege spot #{} cells ({x}, {y}) to ({}, {}): {} kills in {} sieges, {:.1} per siege, {} games",
                    rank + 1,
                    x + cells - 1,
                    y + cells - 1,
                    spot.kills,
                    spot.sieges,
                    spot.kills_per_siege(),
                    spot.games,
                )),
                DisplayInfoOnClick,
                mesh.clone(),
                material,
                // The map x is the world z, as for the terrain.
                Transform::from_xyz(
                    map_y * MAP_SCALE_FACTOR,
                    ground + height / 2.,
                    map_x * MAP_SCALE_FACTOR,
                )
                .with_scale(Vec3::new(1., height, 1.)),
            ))
            .observe(update_info_on_click);
    }
}

fn spawn_legend(mut commands: Commands, spots: Res<SiegeSpots>) {
    let best = spots
        .spots
        .first()
        .map(|spot| {
            let (x, y) = (spot.x * spots.spot_cells, spot.y * spots.spot_cells);
            format!(
                "\nMost kills: cells ({x}, {y}), {} kills in {} sieges",
                spot.kills, spot.sieges
            )
        })
        .unwrap_or_default();
    commands.spawn((
        Text::new(format!(
            "Siege Tank spots of {}\n{} sieges in {} games, {} kills\nHeight: kills, yellow to red: kills per siege{best}",
            spots.map_title, spots.sieges, spots.games, spots.kills,
        )),
        TextFont {
            font_size: bevy::prelude::FontSize::Px(12.),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(40),
            right: px(12),
            ..default()
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_the_spots_of_swarmy() {
        let json = r#"{"map_title":"Taito Citadel LE","spot_cells":4,"games":16,"sieges":74,"kills":156,
            "spots":[{"x":36,"y":12,"sieges":4,"kills":21,"games":2}]}"#;
        let spots: SiegeSpots = serde_json::from_str(json).unwrap();
        assert_eq!(spots.spots[0].kills, 21);
        assert_eq!(spots.center(&spots.spots[0]), (146., 50.));
        assert_eq!(spots.spots[0].kills_per_siege(), 5.25);
    }
}
