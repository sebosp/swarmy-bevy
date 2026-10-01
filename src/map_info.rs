//! Information provided by the MapInfo file.
//!
use crate::{map_plugin::MapInfoResource, t3_height_map::T3HeightMapResource};
use bevy::color::palettes;
use bevy::prelude::*;

/// Shows the basic MapInfo data.
pub fn show_map_info(
    mut commands: Commands,
    map_info: Res<MapInfoResource>,
    t3_height_map: Res<T3HeightMapResource>,
) {
    commands.spawn((
        Text::new(format!(
            "{} - {}\n\
                MapInfo Dimensions: {} - {}\n\
                TerrainHeight Dimensions: {} - {}",
            map_info.theme,
            map_info.tile_set,
            map_info.playable_dimensions.x,
            map_info.playable_dimensions.y,
            t3_height_map.width,
            t3_height_map.height
        )),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            right: px(12),
            ..default()
        },
        TextColor(Color::from(palettes::css::GREEN)),
        TextLayout::default().with_justify(Justify::Right),
        TextFont {
            font_size: bevy::prelude::FontSize::Px(14.),
            ..default()
        },
    ));
}
