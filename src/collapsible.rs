//! The sections that fold: a header with an arrow (down when open, right when folded) and a
//! body, clicking the header folds or unfolds the body. The arrows are drawn images, the default
//! font has no arrows.

use bevy::asset::RenderAssetUsages;
use bevy::color::palettes;
use bevy::prelude::*;

pub struct CollapsiblePlugin;

impl Plugin for CollapsiblePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArrowImages>()
            .add_systems(Update, show_collapsed);
    }
}

/// A section that folds, open unless `collapsed`.
#[derive(Component, Debug, Default, Clone)]
pub struct Collapsible {
    pub collapsed: bool,
}

/// The header of a [`Collapsible`], clicking it folds or unfolds the section.
#[derive(Component, Debug, Default, Clone)]
pub struct CollapsibleHeader;

/// The part of a [`Collapsible`] hidden when it is folded.
#[derive(Component, Debug, Default, Clone)]
pub struct CollapsibleBody;

/// The arrow of a [`Collapsible`]'s header.
#[derive(Component, Debug, Default, Clone)]
#[require(ImageNode)]
pub struct CollapseArrow;

/// The size of the arrows in the headers.
pub const ARROW_SIZE: f32 = 12.;

/// Which way a triangle points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pointing {
    Up,
    Down,
    Right,
}

#[derive(Resource)]
struct ArrowImages {
    open: Handle<Image>,
    folded: Handle<Image>,
}

impl FromWorld for ArrowImages {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        let color = palettes::tailwind::GRAY_300;
        Self {
            open: images.add(triangle_image(Pointing::Down, color)),
            folded: images.add(triangle_image(Pointing::Right, color)),
        }
    }
}

/// Folds or unfolds the section of the clicked header, the click can be on the arrow or the title.
pub fn toggle_collapsible(
    click: On<PointerClick>,
    headers: Query<(), With<CollapsibleHeader>>,
    parents: Query<&ChildOf>,
    mut sections: Query<&mut Collapsible>,
) {
    let Some(header) = std::iter::once(click.entity)
        .chain(parents.iter_ancestors(click.entity))
        .find(|entity| headers.contains(*entity))
    else {
        return;
    };
    let Some(section) = parents
        .iter_ancestors(header)
        .find(|entity| sections.contains(*entity))
    else {
        return;
    };
    if let Ok(mut section) = sections.get_mut(section) {
        section.collapsed = !section.collapsed;
    }
}

/// Hides the bodies of the folded sections and turns their arrows.
fn show_collapsed(
    sections: Query<(Entity, &Collapsible), Changed<Collapsible>>,
    new_arrows: Query<Entity, Added<CollapseArrow>>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    all_sections: Query<&Collapsible>,
    mut bodies: Query<&mut Node, With<CollapsibleBody>>,
    mut arrows: Query<&mut ImageNode, With<CollapseArrow>>,
    images: Res<ArrowImages>,
) {
    let arrow_image = |collapsed: bool| {
        if collapsed {
            images.folded.clone()
        } else {
            images.open.clone()
        }
    };
    for (section, collapsible) in &sections {
        for entity in children.iter_descendants(section) {
            if let Ok(mut body) = bodies.get_mut(entity) {
                body.display = if collapsible.collapsed {
                    Display::None
                } else {
                    Display::Flex
                };
            }
            if let Ok(mut arrow) = arrows.get_mut(entity) {
                arrow.image = arrow_image(collapsible.collapsed);
            }
        }
    }
    // The arrows spawned after their section changed.
    for arrow in &new_arrows {
        let collapsed = parents
            .iter_ancestors(arrow)
            .find_map(|entity| all_sections.get(entity).ok())
            .is_some_and(|section| section.collapsed);
        if let Ok(mut image) = arrows.get_mut(arrow) {
            image.image = arrow_image(collapsed);
        }
    }
}

/// A triangle pointing up, down or right in a square image, transparent around it.
pub fn triangle_image(pointing: Pointing, color: Srgba) -> Image {
    const SIZE: usize = 32;
    let [r, g, b, a] = color.to_u8_array();
    let mut data = vec![0u8; SIZE * SIZE * 4];
    for row in 0..SIZE {
        for column in 0..SIZE {
            // The distance from the point along the triangle, and across it.
            let (along, across) = match pointing {
                Pointing::Up => (row, column),
                Pointing::Down => (SIZE - 1 - row, column),
                Pointing::Right => (SIZE - 1 - column, row),
            };
            let half = (along as f32 + 1.) / 2.;
            let distance = (across as f32 + 0.5 - SIZE as f32 / 2.).abs();
            if distance <= half {
                let idx = (row * SIZE + column) * 4;
                data[idx..idx + 4].copy_from_slice(&[r, g, b, a]);
            }
        }
    }
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: SIZE as u32,
            height: SIZE as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_draws_the_triangles() {
        let alpha = |image: &Image, row: usize, column: usize| {
            image.data.as_ref().unwrap()[(row * 32 + column) * 4 + 3]
        };
        let up = triangle_image(Pointing::Up, palettes::tailwind::GREEN_500);
        // The point at the top, the base at the bottom, transparent around.
        assert_eq!(alpha(&up, 0, 0), 0);
        assert!(alpha(&up, 0, 16) > 0);
        assert!(alpha(&up, 31, 1) > 0);
        let down = triangle_image(Pointing::Down, palettes::tailwind::RED_500);
        assert!(alpha(&down, 31, 16) > 0);
        assert_eq!(alpha(&down, 31, 0), 0);
        let right = triangle_image(Pointing::Right, palettes::tailwind::GRAY_300);
        // The base at the left, the point at the right.
        assert!(alpha(&right, 1, 0) > 0);
        assert!(alpha(&right, 16, 31) > 0);
        assert_eq!(alpha(&right, 0, 31), 0);
    }
}
