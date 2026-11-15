//! The terrain painted with its textures: the 8 textures the map blends (`t3Terrain.xml`) and
//! their weights at each point (`t3TextureMasks`), read by s2protocol. The textures of the game
//! are not in the map caches: each one is drawn with a free texture of the same kind of ground
//! (Poly Haven, CC0, see [`materials`]), tinted with the color of the texture in the minimap.
//!
//! Read in the background the first time the View menu's Terrain Textures is on (by default for a
//! map, not for a replay: its bars stand out on the gray terrain), the Poly Haven textures are
//! downloaded once into the cache directory. Without them the layers are drawn in their colors.

pub mod materials;
pub mod polyhaven;

use crate::MAP_SCALE_FACTOR;
use crate::cache_objects::RockTexture;
use crate::cli::CliParams;
use crate::t3_height_map::TerrainChunk;
use crate::t3_terrain::RampComponent;
use bevy::asset::RenderAssetUsages;
use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat, TextureViewDescriptor,
    TextureViewDimension,
};
use bevy::shader::ShaderRef;
use materials::{GroundKind, MAX_MINIMAP_FIT_ERROR};
use polyhaven::{DETAIL_SIZE, DetailLayer};
use s2protocol::cache_handles::t3_terrain::TERRAIN_BLEND_LAYERS;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const SHADER_PATH: &str = "embedded://swarmy_bevy/terrain_splat.wesl";

/// The points of the masks per map cell on the GPU (the masks have 8).
const WEIGHT_POINTS_PER_CELL: usize = 4;

/// The map cells a texture covers before it repeats.
const CELLS_PER_TEXTURE: f32 = 6.;

/// The material of the terrain cells and ramps, see `terrain_splat.wesl`.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainSplatMaterial {
    #[uniform(0)]
    pub splat: TerrainSplatUniform,
    #[texture(1)]
    #[sampler(2)]
    pub weights_low: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub weights_high: Handle<Image>,
    #[texture(5, dimension = "2d_array")]
    #[sampler(6)]
    pub details: Handle<Image>,
}

#[derive(ShaderType, Debug, Clone, Default)]
pub struct TerrainSplatUniform {
    pub tints: [Vec4; TERRAIN_BLEND_LAYERS],
    pub map: Vec4,
    pub sun: Vec4,
}

impl Material for TerrainSplatMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }
}

/// Whether the terrain is painted with its textures, the View menu changes it.
#[derive(Resource, Default)]
pub struct TerrainTexturesState {
    pub enabled: bool,
    /// The material once read, None before or when the map has no textures.
    pub material: Option<Handle<TerrainSplatMaterial>>,
    /// The layers of the map, for the help: the texture name and the kind of ground drawn.
    pub layers: Vec<(String, GroundKind)>,
    /// The textures could not be read, they are not tried again.
    pub failed: bool,
    /// The material of the rock of the geysers and mineral fields, None before or when its texture
    /// could not be downloaded.
    pub rock: Option<Handle<StandardMaterial>>,
    /// The textures were read (or failed to), they are not read again.
    loaded: bool,
    loading: Option<Arc<Mutex<Option<Prepared>>>>,
}

/// The texture of the rock of the geysers and of the ore of the mineral fields.
const ROCK_TEXTURE: &str = "rock_face_03";

/// What the background thread reads: the terrain, and the image of the rock apart (a map without
/// terrain textures still has its rocks textured).
struct Prepared {
    terrain: Result<PreparedTerrain, String>,
    rock: Option<Image>,
}

/// The material a rock keeps while it is painted with its texture.
#[derive(Component)]
pub struct PlainRockMaterial(Handle<StandardMaterial>);

/// The standard material of a terrain cell or a ramp while it is painted with its textures.
#[derive(Component)]
pub struct PlainTerrainMaterial(Handle<StandardMaterial>);

/// Marks the View menu checkbox, checked at the start when the textures are on.
#[derive(Component, Default, Clone)]
pub struct TerrainTexturesCheckbox;

/// What the background thread prepares, the images are made on the main thread.
struct PreparedTerrain {
    weights_size: (usize, usize),
    weights: [Vec<u8>; 2],
    cells: (f32, f32),
    details: Vec<u8>,
    mip_levels: u32,
    tints: Vec<[f32; 3]>,
    layers: Vec<(String, GroundKind)>,
}

pub struct TerrainTexturesPlugin {
    pub enabled: bool,
}

impl Plugin for TerrainTexturesPlugin {
    fn build(&self, app: &mut App) {
        app.world()
            .resource::<EmbeddedAssetRegistry>()
            .insert_asset(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("src/terrain_textures/terrain_splat.wesl"),
                Path::new("swarmy_bevy/terrain_splat.wesl"),
                include_bytes!("terrain_splat.wesl").as_slice(),
            );
        app.add_plugins(MaterialPlugin::<TerrainSplatMaterial>::default())
            .insert_resource(TerrainTexturesState {
                enabled: self.enabled,
                ..default()
            })
            .add_systems(
                Update,
                (
                    check_the_checkbox,
                    start_loading,
                    finish_loading,
                    paint_terrain,
                    paint_rocks,
                )
                    .chain(),
            );
    }
}

fn check_the_checkbox(
    mut commands: Commands,
    state: Res<TerrainTexturesState>,
    checkboxes: Query<Entity, Added<TerrainTexturesCheckbox>>,
) {
    for checkbox in &checkboxes {
        if state.enabled {
            commands.entity(checkbox).insert(bevy::ui::Checked);
        }
    }
}

/// Starts reading the textures the first time they are on.
fn start_loading(mut state: ResMut<TerrainTexturesState>, cli: Option<Res<CliParams>>) {
    if !state.enabled || state.loaded || state.loading.is_some() {
        return;
    }
    let Some(cli) = cli else {
        return;
    };
    let slot = Arc::new(Mutex::new(None));
    state.loading = Some(slot.clone());
    let (path, ids) = (cli.path.clone(), cli.ids.clone());
    std::thread::spawn(move || {
        let ids: Vec<String> = ids.split(',').map(str::to_string).collect();
        let prepared = Prepared {
            rock: rock_image(),
            terrain: prepare_terrain(&path, &ids),
        };
        if let Ok(mut slot) = slot.lock() {
            *slot = Some(prepared);
        }
    });
}

/// The image of the rocks, None when it cannot be downloaded.
fn rock_image() -> Option<Image> {
    polyhaven::diffuse_jpg(ROCK_TEXTURE)
        .and_then(|jpg| {
            Image::from_buffer(
                &jpg,
                bevy::image::ImageType::Extension("jpg"),
                bevy::image::CompressedImageFormats::NONE,
                true,
                ImageSampler::Default,
                RenderAssetUsages::RENDER_WORLD,
            )
            .map_err(|err| err.to_string())
        })
        .inspect_err(|err| tracing::warn!("Rock texture {ROCK_TEXTURE}: {err}"))
        .ok()
}

/// Reads the textures of the map, its colors and the Poly Haven textures, off the main thread.
fn prepare_terrain(path: &str, ids: &[String]) -> Result<PreparedTerrain, String> {
    let mut caches = s2protocol::cache_handles::CacheCollection::new(path.to_string());
    caches.add_cache_ids(ids);
    let terrain = caches
        .build_terrain_textures(ids)
        .map_err(|err| err.to_string())?;
    let fit = terrain
        .layer_colors
        .as_ref()
        .filter(|colors| colors.mean_abs_error <= MAX_MINIMAP_FIT_ERROR);
    if let Some(colors) = &terrain.layer_colors {
        tracing::info!(
            "Terrain {}: minimap colors {} (error {:.1})",
            terrain.tile_set,
            if fit.is_some() {
                "used"
            } else {
                "not used, drawn by hand"
            },
            colors.mean_abs_error
        );
    }
    let minimap_colors: Vec<Option<[u8; 3]>> = (0..TERRAIN_BLEND_LAYERS)
        .map(|layer| fit.and_then(|colors| colors.colors[layer]))
        .collect();
    let shares: Vec<f32> = (0..TERRAIN_BLEND_LAYERS)
        .map(|layer| {
            terrain
                .layer_colors
                .as_ref()
                .map_or(0.125, |c| c.shares[layer])
        })
        .collect();
    let kinds: Vec<GroundKind> = (0..TERRAIN_BLEND_LAYERS)
        .map(|layer| {
            let color = terrain.layer_colors.as_ref().and_then(|c| c.colors[layer]);
            materials::ground_kind(&terrain.layer_names[layer], color)
        })
        .collect();
    // One texture per kind of ground, downloaded once.
    let mut details: Vec<(GroundKind, DetailLayer)> = vec![];
    for kind in &kinds {
        if details.iter().any(|(k, _)| k == kind) {
            continue;
        }
        let layer = polyhaven::diffuse_jpg(kind.polyhaven_id())
            .and_then(|jpg| DetailLayer::from_image_bytes(&jpg, "jpg"))
            .unwrap_or_else(|err| {
                tracing::warn!("Terrain texture {}: {err}", kind.polyhaven_id());
                let c = kind.default_color();
                DetailLayer::flat([
                    materials::srgb_to_linear(c[0]),
                    materials::srgb_to_linear(c[1]),
                    materials::srgb_to_linear(c[2]),
                ])
            });
        details.push((*kind, layer));
    }
    let detail_of = |kind: &GroundKind| &details.iter().find(|(k, _)| k == kind).unwrap().1;
    let fallback: Vec<[f32; 3]> = kinds.iter().map(|kind| detail_of(kind).mean).collect();
    let tints = materials::layer_tints(&minimap_colors, &shares, &fallback);
    let mut all_details = vec![];
    let mut mip_levels = 1;
    for kind in &kinds {
        let (chain, levels) = polyhaven::mip_chain(&detail_of(kind).pixels, DETAIL_SIZE);
        all_details.extend_from_slice(&chain);
        mip_levels = levels;
    }
    // The masks cover the whole map, 8 points per cell.
    const MASK_POINTS_PER_CELL: usize = 8;
    let step = (MASK_POINTS_PER_CELL / WEIGHT_POINTS_PER_CELL).max(1);
    let (width, height, weights) = terrain.masks.to_rgba_images(step);
    let cells = (
        (terrain.masks.width / MASK_POINTS_PER_CELL) as f32,
        (terrain.masks.height / MASK_POINTS_PER_CELL) as f32,
    );
    let layers = terrain
        .layer_names
        .iter()
        .cloned()
        .zip(kinds.iter().copied())
        .filter(|(name, _)| !name.is_empty())
        .collect();
    Ok(PreparedTerrain {
        weights_size: (width, height),
        weights,
        cells,
        details: all_details,
        mip_levels,
        tints,
        layers,
    })
}

/// Makes the images and the material once the background thread is done.
fn finish_loading(
    mut state: ResMut<TerrainTexturesState>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<TerrainSplatMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(slot) = &state.loading else {
        return;
    };
    let Some(prepared) = slot.lock().ok().and_then(|mut slot| slot.take()) else {
        return;
    };
    state.loading = None;
    state.loaded = true;
    if let Some(rock) = prepared.rock {
        // Unlit, as the map graphics: the scene has a single weak light. The ore of the mineral
        // fields has its light baked in its vertex colors, see [`crate::baked_light`].
        state.rock = Some(standard_materials.add(StandardMaterial {
            base_color: Color::srgb(0.62, 0.58, 0.52),
            base_color_texture: Some(images.add(rock)),
            unlit: true,
            ..default()
        }));
    }
    let prepared = match prepared.terrain {
        Ok(prepared) => prepared,
        Err(err) => {
            tracing::warn!("No terrain textures: {err}");
            state.failed = true;
            return;
        }
    };
    let weights_sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        ..default()
    });
    let (width, height) = prepared.weights_size;
    let [low, high] = prepared.weights;
    let weight_image = |data: Vec<u8>| {
        let mut image = Image::new(
            Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = weights_sampler.clone();
        image
    };
    let weights_low = images.add(weight_image(low));
    let weights_high = images.add(weight_image(high));
    let mut details = Image::new_uninit(
        Extent3d {
            width: DETAIL_SIZE as u32,
            height: DETAIL_SIZE as u32,
            depth_or_array_layers: TERRAIN_BLEND_LAYERS as u32,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    details.data = Some(prepared.details);
    details.texture_descriptor.mip_level_count = prepared.mip_levels;
    details.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    details.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        anisotropy_clamp: 8,
        ..default()
    });
    let details = images.add(details);
    let mut tints = [Vec4::ZERO; TERRAIN_BLEND_LAYERS];
    for (tint, color) in tints.iter_mut().zip(&prepared.tints) {
        *tint = Vec4::new(color[0], color[1], color[2], 1.);
    }
    let material = materials.add(TerrainSplatMaterial {
        splat: TerrainSplatUniform {
            tints,
            map: Vec4::new(
                MAP_SCALE_FACTOR,
                prepared.cells.0,
                prepared.cells.1,
                CELLS_PER_TEXTURE,
            ),
            sun: Vec4::new(0.4, 1.0, 0.3, 0.45),
        },
        weights_low,
        weights_high,
        details,
    });
    for (name, kind) in &prepared.layers {
        tracing::info!("Terrain layer {name}: {kind:?} ({})", kind.polyhaven_id());
    }
    state.layers = prepared.layers;
    state.material = Some(material);
}

type TerrainPart = Or<(With<TerrainChunk>, With<RampComponent>)>;

/// Paints the cells and ramps with the textures, or back in their colors.
fn paint_terrain(
    mut commands: Commands,
    state: Res<TerrainTexturesState>,
    plain: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>),
        (TerrainPart, Without<PlainTerrainMaterial>),
    >,
    painted: Query<(Entity, &PlainTerrainMaterial)>,
) {
    match (&state.material, state.enabled) {
        (Some(material), true) => {
            for (entity, standard) in &plain {
                commands
                    .entity(entity)
                    .insert((
                        PlainTerrainMaterial(standard.0.clone()),
                        MeshMaterial3d(material.clone()),
                    ))
                    .remove::<MeshMaterial3d<StandardMaterial>>();
            }
        }
        _ => {
            for (entity, standard) in &painted {
                commands
                    .entity(entity)
                    .insert(MeshMaterial3d(standard.0.clone()))
                    .remove::<(PlainTerrainMaterial, MeshMaterial3d<TerrainSplatMaterial>)>();
            }
        }
    }
}

/// Paints the geysers and the ore of the mineral fields with the rock, or back in their colors.
fn paint_rocks(
    mut commands: Commands,
    state: Res<TerrainTexturesState>,
    plain: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>),
        (With<RockTexture>, Without<PlainRockMaterial>),
    >,
    painted: Query<(Entity, &PlainRockMaterial)>,
) {
    match (&state.rock, state.enabled) {
        (Some(rock), true) => {
            for (entity, standard) in &plain {
                commands.entity(entity).insert((
                    PlainRockMaterial(standard.0.clone()),
                    MeshMaterial3d(rock.clone()),
                ));
            }
        }
        _ => {
            for (entity, standard) in &painted {
                commands
                    .entity(entity)
                    .insert(MeshMaterial3d(standard.0.clone()))
                    .remove::<PlainRockMaterial>();
            }
        }
    }
}
