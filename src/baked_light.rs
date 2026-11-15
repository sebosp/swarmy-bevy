//! A light baked into the vertex colors of a mesh: the scene has a single weak light, the models
//! drawn unlit would be flat. An unlit material multiplies its color by the vertex colors, the
//! faces turned to the sun are bright, the others darker, so that the shapes read.

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;

/// Where the baked light comes from: mostly above, a little from the side.
const SUN: Vec3 = Vec3::new(0.35, 1.0, 0.25);

/// The brightness of a face turned away from the sun.
pub const AMBIENT: f32 = 0.42;

/// The brightness of a normal, from [`AMBIENT`] (away from the sun) to 1 (facing it).
pub fn brightness(normal: Vec3) -> f32 {
    let facing = normal.normalize_or_zero().dot(SUN.normalize()).max(0.);
    AMBIENT + (1. - AMBIENT) * facing
}

/// Colors the vertices by their normals, the mesh keeps its shared vertices (smooth shading).
pub fn bake_smooth_light(mut mesh: Mesh) -> Mesh {
    let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    else {
        return mesh;
    };
    let colors: Vec<[f32; 4]> = normals
        .iter()
        .map(|normal| {
            let light = brightness(Vec3::from_array(*normal));
            [light, light, light, 1.]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh
}

/// Gives each triangle its own vertices and normal, then colors them: the facets of a crystal or a
/// rock each have one brightness.
pub fn bake_flat_light(mut mesh: Mesh) -> Mesh {
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    bake_smooth_light(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_lights_the_faces_turned_to_the_sun() {
        assert!(brightness(Vec3::Y) > 0.9);
        assert_eq!(brightness(Vec3::NEG_Y), AMBIENT);
        assert!(brightness(Vec3::X) > AMBIENT && brightness(Vec3::X) < brightness(Vec3::Y));
    }

    #[test]
    fn it_colors_each_facet_once() {
        let mesh = bake_flat_light(Cuboid::default().mesh().build());
        let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("no colors");
        };
        // 12 triangles of 3 vertices, the top ones the brightest.
        assert_eq!(colors.len(), 36);
        let top = colors.iter().map(|color| color[0]).fold(0., f32::max);
        assert_eq!(top, brightness(Vec3::Y));
    }
}
