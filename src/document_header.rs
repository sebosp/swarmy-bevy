//! Displays information contained in the DocumentHeader

use bevy::{color::palettes, prelude::*};

use crate::map_plugin::DocumentHeaderResource;
pub fn show_document_header(mut commands: Commands, document_header: Res<DocumentHeaderResource>) {
    // Print DocumentHeader
    commands.spawn((
        Text::new(format!(
            "{} - {}\n\
                {}\n\
            {} - {}\n",
            document_header.name,
            document_header.mod_info,
            document_header.description_long,
            document_header.some_epoch_1,
            document_header.some_epoch_2,
        )),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            right: px(12),
            ..default()
        },
        TextColor(Color::from(palettes::css::GOLD)),
        TextLayout::default().with_justify(Justify::Right),
        TextFont {
            font_size: bevy::prelude::FontSize::Px(14.),
            ..default()
        },
    ));
}
