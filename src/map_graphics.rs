//! The models and textures a map carries in its caches, drawn where the `Objects` file places them,
//! on top of the terrain. They are read the first time they are enabled in the View menu: the
//! window stays light for the ones who do not use them.

use crate::MAP_SCALE_FACTOR;
use crate::cli::CliParams;
use crate::swarmy_feathers::{DisplayInfoOnClick, update_info_on_click};
use crate::t3_height_map::{T3HeightMapResource, terrain_top};
use bevy::asset::RenderAssetUsages;
use bevy::image::{
    CompressedImageFormats, ImageAddressMode, ImageSampler, ImageSamplerDescriptor, ImageType,
};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use s2protocol::cache_handles::graphics::{MapGraphics, normalized_path, read_map_graphics};
use s2protocol::cache_handles::m3::{M3Model, M3Region};
use std::collections::HashMap;

/// Cutouts such as leaves are drawn with holes, the rest of a texture is opaque.
const ALPHA_CUTOFF: f32 = 0.3;
/// The gray of a model whose texture is one of the game's, not in the caches.
const MISSING_TEXTURE_GRAY: f32 = 0.45;

/// An entity drawn from a model of the map caches.
#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
pub struct MapGraphicComponent;

/// Whether the graphics are shown, the View menu changes it.
#[derive(Resource, Default, Debug)]
pub struct MapGraphicsState {
    pub enabled: bool,
    pub loaded: bool,
}

/// The vertices of a region of a model in the axes of the map (Y is up), the indices start at its
/// first vertex so that the meshes only hold their own vertices.
#[derive(Debug, Default, PartialEq)]
pub struct RegionGeometry {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

/// The axes of the map: the objects are drawn at (y, height, x).
fn to_map_axes([x, y, z]: [f32; 3]) -> [f32; 3] {
    [y, z, x]
}

pub fn region_geometry(model: &M3Model, region: &M3Region) -> RegionGeometry {
    let mut geometry = RegionGeometry::default();
    let mut remapped: HashMap<u32, u32> = HashMap::new();
    for index in &region.indices {
        let Some(vertex) = model.vertices.get(*index as usize) else {
            continue;
        };
        let new_index = *remapped.entry(*index).or_insert_with(|| {
            geometry.positions.push(to_map_axes(vertex.position));
            geometry.normals.push(to_map_axes(vertex.normal));
            geometry.uvs.push(vertex.uv);
            geometry.positions.len() as u32 - 1
        });
        geometry.indices.push(new_index);
    }
    geometry
}

fn region_mesh(geometry: RegionGeometry) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, geometry.positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, geometry.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, geometry.uvs)
    .with_inserted_indices(Indices::U32(geometry.indices))
}

/// The material of a texture path, None for the regions without one. They are shared, and unlit
/// so that the images are readable wherever the camera goes.
fn graphic_material(
    texture: Option<&String>,
    graphics: &MapGraphics,
    images: &mut Assets<Image>,
) -> StandardMaterial {
    let image = texture.and_then(|path| {
        let dds = graphics.textures.get(&normalized_path(path))?;
        Image::from_buffer(
            dds,
            ImageType::Extension("dds"),
            CompressedImageFormats::BC,
            true,
            ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..ImageSamplerDescriptor::linear()
            }),
            RenderAssetUsages::default(),
        )
        .inspect_err(|err| tracing::warn!("Unable to decode the texture {path}: {err:?}"))
        .ok()
    });
    let has_image = image.is_some();
    StandardMaterial {
        base_color: if has_image {
            Color::WHITE
        } else {
            Color::srgb(
                MISSING_TEXTURE_GRAY,
                MISSING_TEXTURE_GRAY,
                MISSING_TEXTURE_GRAY,
            )
        },
        base_color_texture: image.map(|image| images.add(image)),
        unlit: true,
        double_sided: true,
        cull_mode: None,
        alpha_mode: if has_image {
            AlphaMode::Mask(ALPHA_CUTOFF)
        } else {
            AlphaMode::Opaque
        },
        ..default()
    }
}

/// Reads and draws the graphics the first time they are enabled.
pub fn load_map_graphics(
    mut commands: Commands,
    mut state: ResMut<MapGraphicsState>,
    cli_params: Res<CliParams>,
    height_map: Res<T3HeightMapResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !state.enabled || state.loaded {
        return;
    }
    state.loaded = true;
    let cache_ids: Vec<String> = cli_params.ids.split(',').map(str::to_string).collect();
    let graphics = match read_map_graphics(&cli_params.path, &cache_ids) {
        Ok(graphics) => graphics,
        Err(err) => {
            tracing::error!("Unable to read the graphics of the map: {err:?}");
            return;
        }
    };
    tracing::info!("The graphics of the map: {:?}", graphics.counts);
    // The material of each texture is shared, the meshes of each model too.
    let mut shared_materials: HashMap<Option<String>, Handle<StandardMaterial>> = HashMap::new();
    let mut model_parts: HashMap<&str, Vec<(Handle<Mesh>, Handle<StandardMaterial>)>> =
        HashMap::new();
    for (key, model) in &graphics.models {
        let parts = model
            .regions
            .iter()
            .map(|region| {
                let material = shared_materials
                    .entry(region.diffuse.as_deref().map(normalized_path))
                    .or_insert_with(|| {
                        materials.add(graphic_material(
                            region.diffuse.as_ref(),
                            &graphics,
                            &mut images,
                        ))
                    })
                    .clone();
                (
                    meshes.add(region_mesh(region_geometry(model, region))),
                    material,
                )
            })
            .collect();
        model_parts.insert(key.as_str(), parts);
    }
    for placed in &graphics.placements {
        let Some(parts) = model_parts.get(placed.model.as_str()) else {
            continue;
        };
        let [x, y, _] = placed.position;
        let ground = height_map
            .height_at(x as i32, y as i32)
            .map_or(0., |height| terrain_top(f32::from(height)));
        let transform = Transform::from_xyz(MAP_SCALE_FACTOR * y, ground, MAP_SCALE_FACTOR * x)
            .with_rotation(Quat::from_rotation_y(placed.rotation))
            .with_scale(Vec3::from(to_map_axes(placed.scale)) * MAP_SCALE_FACTOR);
        for (mesh, material) in parts {
            commands
                .spawn((
                    MapGraphicComponent,
                    DisplayInfoOnClick,
                    Name::new(format!("{}:{}", placed.kind, placed.id)),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    transform,
                ))
                .observe(update_info_on_click);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s2protocol::cache_handles::m3::M3Vertex;

    #[test]
    fn it_keeps_only_the_vertices_of_the_region_in_the_axes_of_the_map() {
        let vertex = |position| M3Vertex {
            position,
            normal: [0., 0., 1.],
            uv: [0.5, 0.25],
        };
        let model = M3Model {
            vertices: vec![
                vertex([9., 9., 9.]),
                vertex([1., 2., 3.]),
                vertex([4., 5., 6.]),
                vertex([7., 8., 0.]),
            ],
            regions: vec![],
        };
        let region = M3Region {
            indices: vec![1, 2, 3, 3, 2, 1],
            diffuse: None,
        };
        let geometry = region_geometry(&model, &region);
        assert_eq!(geometry.indices, vec![0, 1, 2, 2, 1, 0]);
        assert_eq!(geometry.positions[0], [2., 3., 1.]);
        assert_eq!(geometry.normals[0], [0., 1., 0.]);
        assert_eq!(geometry.positions.len(), 3);
    }

    #[test]
    fn it_ignores_the_vertices_out_of_the_model() {
        let model = M3Model::default();
        let region = M3Region {
            indices: vec![5],
            diffuse: None,
        };
        assert_eq!(region_geometry(&model, &region), RegionGeometry::default());
    }
}
