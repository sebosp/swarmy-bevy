//! The main Map Bevy Plugin
use bevy::prelude::*;
use s2protocol::cache_handles::DOCUMENT_HEADER_FILE_NAME;
use s2protocol::cache_handles::MAP_INFO_FILE_NAME;
use s2protocol::cache_handles::T3_HEIGHT_MAP_FILE_NAME;

use crate::cache_objects;
use crate::cache_objects::PlacedObjectsResource;
use crate::cli::CliParams;
use crate::swarmy_feathers;
use crate::t3_height_map;
use crate::t3_height_map::MapDimension;
use crate::t3_height_map::T3HeightMapResource;
use crate::t3_terrain;
use crate::t3_terrain::T3TerrainResource;
use crate::utils;

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, load_cache_depot_map_resources);
        app.add_systems(Startup, MapPlugin::scene.spawn());
        app.add_systems(Startup, t3_height_map::load_t3_height_map);
        app.add_systems(Startup, cache_objects::doodas::load_object_doodas);
        app.add_systems(Update, cache_objects::load_cache_objects);
        app.add_systems(Update, t3_terrain::load_t3_terrain);
        app.add_systems(Update, utils::update_gizmo_config);
    }
}
impl MapPlugin {
    fn scene() -> impl SceneList {
        bsn_list![swarmy_feathers::init_feathers()]
    }
}

/// Contains errors to be displayed as Feathers when unable to find info in the cache depots, serde
/// issues, etc.
#[derive(Component, Default, Reflect, Debug, Clone)]
pub struct MapPluginError {
    pub kind: String,
    pub err: String,
}

impl MapPluginError {
    fn new(kind: &'static str, err: impl ToString) -> Self {
        Self {
            kind: kind.to_string(),
            err: err.to_string(),
        }
    }
}

/// A resource mirror of s2protocol::cache_handles::map_info::MapInfo
#[derive(Resource, Default, Reflect, Debug, Clone)]
pub struct MapInfoResource {
    pub file_version: i32,
    pub cell_width: usize,
    pub cell_height: usize,
    /// Mostly seen empty?
    pub first_string: String,
    /// Also empty?
    pub second_string: String,
    // Maybe a theme, light Dark/Light/Black?
    pub theme: String,
    // Some name, "Zerus" in the test case, maybe map maker?
    pub tile_set: String,
    pub cell_left: usize,
    pub cell_bottom: usize,
    pub cell_right: usize,
    pub cell_top: usize,
    pub playable_dimensions: MapDimension,
}

impl From<s2protocol::cache_handles::map_info::MapInfo> for MapInfoResource {
    fn from(src: s2protocol::cache_handles::map_info::MapInfo) -> Self {
        let playable_dimensions = MapDimension::from(src.cell_dim_playable());
        Self {
            file_version: src.file_version,
            cell_width: src.cell_width,
            cell_height: src.cell_height,
            first_string: src.first_string.unwrap_or_default(),
            second_string: src.second_string.unwrap_or_default(),
            theme: src.theme,
            tile_set: src.tile_set,
            cell_left: src.cell_left,
            cell_bottom: src.cell_bottom,
            cell_right: src.cell_right,
            cell_top: src.cell_top,
            playable_dimensions,
        }
    }
}

/// A resource mirror of s2protocol::cache_handles::document_header::DocumentHeader
#[derive(Resource, Reflect, Debug, Default, Clone)]
pub struct DocumentHeaderResource {
    pub maybe_dimension_x1: i32,
    pub maybe_dimension_y1: i32,
    // TODO: these are not epochs, probably more like units that are 1000th the value of the
    // maybe_dimension_x1
    pub some_epoch_1: i32,
    pub some_epoch_2: i32,
    pub mod_info: String,
    // Aka the map title.
    pub name: String,
    /// A long description of the map.
    pub description_long: String,
    /// A short description of the map, in the few files I've checked it's empty.
    pub description_short: String,
}

impl From<s2protocol::cache_handles::document_header::DocumentHeader> for DocumentHeaderResource {
    fn from(src: s2protocol::cache_handles::document_header::DocumentHeader) -> Self {
        Self {
            maybe_dimension_x1: src.maybe_dimension_x1,
            maybe_dimension_y1: src.maybe_dimension_y1,
            some_epoch_1: src.some_epoch_1,
            some_epoch_2: src.some_epoch_2,
            mod_info: src.mod_info,
            name: src.name,
            description_long: src.description_long,
            description_short: src.description_short,
        }
    }
}

/// Attempts to load the available resources from the downloaded caches.
pub fn load_cache_depot_map_resources(mut commands: Commands, cli_params: Res<CliParams>) {
    let mut cache_builder =
        s2protocol::cache_handles::CacheCollection::new(cli_params.path.clone());
    let cache_ids = cli_params
        .ids
        .split(",")
        .map(|x| x.to_string())
        .collect::<Vec<String>>();
    cache_builder.add_cache_ids(&cache_ids);
    match cache_builder.build_map_cache(&cache_ids) {
        Ok(map_cache) => {
            let t3_height_map = map_cache.t3_height_map;
            let max_map_dim = t3_height_map.width.max(t3_height_map.height);
            let mut cell_x_y_data: Vec<u8> = Vec::with_capacity(max_map_dim * max_map_dim);
            for _ in 0..(max_map_dim * max_map_dim) {
                cell_x_y_data.push(0);
            }
            for (idx, cell_height) in t3_height_map.data.iter().enumerate() {
                let x = usize::try_from(
                    (t3_height_map.width as i32 - idx as i32).abs() % t3_height_map.width as i32,
                )
                .unwrap();
                let y = usize::try_from(
                    (t3_height_map.width as i32 - idx as i32).abs() / t3_height_map.width as i32,
                )
                .unwrap();
                let target_vec_pos = y as usize * t3_height_map.width as usize + x as usize;
                cell_x_y_data[target_vec_pos] = *cell_height;
            }
            let t3_height_map_res = T3HeightMapResource {
                data: cell_x_y_data,
                width: t3_height_map.width as usize,
                height: t3_height_map.height as usize,
            };
            commands.insert_resource(t3_height_map_res);
            let map_info_res = MapInfoResource::from(map_cache.map_info);
            commands.insert_resource(map_info_res);
            let mut document_header = map_cache.document_header.clone();
            tracing::debug!("docu header: {:?}", document_header);

            // Remove double new lines to save space in the UI.
            let line_len = 80usize;
            let mut desc_lines: Vec<String> = vec![];
            document_header.description_long =
                document_header.description_long.replace("<n/><n/>", "<n/>");
            let chunks = document_header.description_long.split(" ");
            let mut curr_str = String::from("");
            for chunk in chunks {
                if curr_str.len() < line_len {
                    curr_str.push_str(" ");
                    curr_str.push_str(chunk);
                } else {
                    desc_lines.push(curr_str.replace("<n/>", "\n"));
                    curr_str = chunk.to_string();
                }
            }

            desc_lines.push(curr_str.replace("<n/>", "\n"));
            document_header.description_long = desc_lines.join("\n");
            if let Ok(t3_terrain_res) = T3TerrainResource::try_from(map_cache.t3_terrain) {
                commands.insert_resource(t3_terrain_res);
            }
            let document_header_res = DocumentHeaderResource::from(document_header);
            commands.insert_resource(document_header_res);
            let placed_objects = map_cache.placed_objects;
            tracing::debug!("placed objects: {:?}", placed_objects);
            let placed_obj_res: PlacedObjectsResource = placed_objects.into();
            commands.insert_resource(placed_obj_res);
        }
        Err(err) => {
            error!(
                "Unable to collect map cache: {}: {:?}",
                cli_params.path,
                err.to_string(),
            );
            commands.spawn(MapPluginError::new("map_cache", err));
        }
    };
}
