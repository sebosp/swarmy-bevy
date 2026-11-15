//! The material of each terrain texture of a map: the textures of the game are not in the map
//! caches, each one is drawn with a free texture (Poly Haven, CC0) of the same kind, tinted with
//! its color in the minimap.

/// The kinds of ground the terrain textures are drawn as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroundKind {
    Grass,
    Foliage,
    Dirt,
    Sand,
    Rock,
    RockyGround,
    Snow,
    Mud,
    Tiles,
    Brick,
    Metal,
    Concrete,
    Cobblestone,
    RedSoil,
}

impl GroundKind {
    /// The Poly Haven texture (CC0) drawn for the kind, its id in the Poly Haven API.
    pub fn polyhaven_id(self) -> &'static str {
        match self {
            GroundKind::Grass => "aerial_grass_rock",
            GroundKind::Foliage => "forest_leaves_02",
            GroundKind::Dirt => "gravelly_sand",
            GroundKind::Sand => "coast_sand_01",
            GroundKind::Rock => "rock_face_03",
            GroundKind::RockyGround => "aerial_rocks_02",
            GroundKind::Snow => "snow_02",
            GroundKind::Mud => "brown_mud_02",
            GroundKind::Tiles => "large_floor_tiles_02",
            GroundKind::Brick => "stone_brick_wall_001",
            GroundKind::Metal => "metal_plate",
            GroundKind::Concrete => "concrete_floor_worn_001",
            GroundKind::Cobblestone => "cobblestone_floor_04",
            GroundKind::RedSoil => "red_laterite_soil_stones",
        }
    }

    /// The sRGB color of the kind when the map gives none and its texture is not downloaded.
    pub fn default_color(self) -> [u8; 3] {
        match self {
            GroundKind::Grass => [96, 110, 52],
            GroundKind::Foliage => [104, 86, 48],
            GroundKind::Dirt => [128, 100, 70],
            GroundKind::Sand => [170, 152, 120],
            GroundKind::Rock => [120, 108, 96],
            GroundKind::RockyGround => [110, 106, 92],
            GroundKind::Snow => [214, 220, 226],
            GroundKind::Mud => [72, 60, 46],
            GroundKind::Tiles => [128, 132, 136],
            GroundKind::Brick => [110, 96, 80],
            GroundKind::Metal => [96, 100, 104],
            GroundKind::Concrete => [118, 118, 116],
            GroundKind::Cobblestone => [130, 124, 112],
            GroundKind::RedSoil => [130, 70, 44],
        }
    }
}

/// The words of the texture names of the game and the kind they are drawn as, the first that
/// matches wins (cobblestone before stone, bricks before tiles).
const NAME_WORDS: [(&str, GroundKind); 44] = [
    ("cobble", GroundKind::Cobblestone),
    ("brick", GroundKind::Brick),
    ("masonry", GroundKind::Brick),
    ("tile", GroundKind::Tiles),
    ("temple", GroundKind::Tiles),
    ("platform", GroundKind::Tiles),
    ("floor", GroundKind::Tiles),
    ("panel", GroundKind::Tiles),
    ("pave", GroundKind::Tiles),
    ("metal", GroundKind::Metal),
    ("plate", GroundKind::Metal),
    ("hull", GroundKind::Metal),
    ("grate", GroundKind::Metal),
    ("steel", GroundKind::Metal),
    ("concrete", GroundKind::Concrete),
    ("road", GroundKind::Concrete),
    ("street", GroundKind::Concrete),
    ("asphalt", GroundKind::Concrete),
    ("city", GroundKind::Concrete),
    ("grass", GroundKind::Grass),
    ("lawn", GroundKind::Grass),
    ("moss", GroundKind::Grass),
    ("foliage", GroundKind::Foliage),
    ("brush", GroundKind::Foliage),
    ("leaf", GroundKind::Foliage),
    ("leaves", GroundKind::Foliage),
    ("jungle", GroundKind::Foliage),
    ("root", GroundKind::Foliage),
    ("snow", GroundKind::Snow),
    ("ice", GroundKind::Snow),
    ("frost", GroundKind::Snow),
    ("sand", GroundKind::Sand),
    ("dune", GroundKind::Sand),
    ("beach", GroundKind::Sand),
    ("mud", GroundKind::Mud),
    ("swamp", GroundKind::Mud),
    ("creep", GroundKind::Mud),
    ("dirt", GroundKind::Dirt),
    ("soil", GroundKind::Dirt),
    ("ground", GroundKind::Dirt),
    ("ash", GroundKind::Dirt),
    ("gravel", GroundKind::RockyGround),
    ("rubble", GroundKind::RockyGround),
    ("rock", GroundKind::Rock),
];

/// The words for rock that are also parts of other words, checked after the list above.
const ROCK_WORDS: [&str; 6] = ["stone", "cliff", "shale", "crack", "boulder", "lava"];

/// The kind of ground of a terrain texture of the game, by the words of its name, else by its color
/// in the minimap, else dirt.
pub fn ground_kind(name: &str, color: Option<[u8; 3]>) -> GroundKind {
    let lower = name.to_lowercase();
    if let Some((_, kind)) = NAME_WORDS.iter().find(|(word, _)| lower.contains(word)) {
        return *kind;
    }
    if ROCK_WORDS.iter().any(|word| lower.contains(word)) {
        return GroundKind::Rock;
    }
    color.map(ground_kind_by_color).unwrap_or(GroundKind::Dirt)
}

/// The kind of ground of a color: grays are rock (snow when light), greens grass, oranges and
/// browns sand or dirt, reds red soil, blues and purples rock (crystals, the void).
pub fn ground_kind_by_color([r, g, b]: [u8; 3]) -> GroundKind {
    let (r, g, b) = (r as f32 / 255., g as f32 / 255., b as f32 / 255.);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let value = max;
    let saturation = if max > 0. { (max - min) / max } else { 0. };
    if saturation < 0.18 {
        return if value > 0.65 {
            GroundKind::Snow
        } else {
            GroundKind::Rock
        };
    }
    let hue = if max == r {
        60. * ((g - b) / (max - min)).rem_euclid(6.)
    } else if max == g {
        60. * ((b - r) / (max - min) + 2.)
    } else {
        60. * ((r - g) / (max - min) + 4.)
    };
    match hue {
        h if !(15. ..330.).contains(&h) => GroundKind::RedSoil,
        h if h < 55. => {
            if value > 0.45 {
                GroundKind::Sand
            } else {
                GroundKind::Dirt
            }
        }
        h if h < 170. => GroundKind::Grass,
        _ => GroundKind::Rock,
    }
}

/// Above this mean difference between the minimap and the layers painted with their colors, the
/// minimap is drawn by hand: its colors are not used.
pub const MAX_MINIMAP_FIT_ERROR: f32 = 20.;

/// The mean linear luminance the colors of the minimap are brightened to: the minimap is darker
/// than the terrain in the game.
const TARGET_LUMINANCE: f32 = 0.2;

pub fn srgb_to_linear(channel: u8) -> f32 {
    let c = channel as f32 / 255.;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(rgb: [f32; 3]) -> f32 {
    0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
}

/// The linear tints of the layers: the colors of the minimap brightened together so that their
/// mean, weighted by the share of each layer, has `TARGET_LUMINANCE`; the layers without one
/// take `fallback` (the mean color of their texture, or of their kind) as it is.
pub fn layer_tints(
    minimap_colors: &[Option<[u8; 3]>],
    shares: &[f32],
    fallback: &[[f32; 3]],
) -> Vec<[f32; 3]> {
    let linear: Vec<Option<[f32; 3]>> = minimap_colors
        .iter()
        .map(|color| {
            color.map(|c| {
                [
                    srgb_to_linear(c[0]),
                    srgb_to_linear(c[1]),
                    srgb_to_linear(c[2]),
                ]
            })
        })
        .collect();
    let (weighted, total) = linear
        .iter()
        .zip(shares)
        .filter_map(|(color, share)| color.map(|c| (luminance(c) * share, *share)))
        .fold((0., 0.), |(l, t), (lum, share)| (l + lum, t + share));
    let gain = if weighted > 0. {
        (TARGET_LUMINANCE * total / weighted).clamp(1., 5.)
    } else {
        1.
    };
    linear
        .iter()
        .zip(fallback)
        .map(|(color, fallback)| match color {
            Some(c) => [
                (c[0] * gain).min(1.),
                (c[1] * gain).min(1.),
                (c[2] * gain).min(1.),
            ],
            None => *fallback,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_the_kind_of_ground_from_the_names() {
        assert_eq!(ground_kind("AgriaGrassGreen", None), GroundKind::Grass);
        assert_eq!(ground_kind("BelShirBricksSmall", None), GroundKind::Brick);
        assert_eq!(ground_kind("BelShirSmallTiles", None), GroundKind::Tiles);
        assert_eq!(ground_kind("CharDunes_5", None), GroundKind::Sand);
        assert_eq!(ground_kind("ShakurasRockCracked", None), GroundKind::Rock);
        assert_eq!(ground_kind("UlaanStone", None), GroundKind::Rock);
        assert_eq!(ground_kind("BWMud", None), GroundKind::Mud);
        assert_eq!(ground_kind("IceWorld3", None), GroundKind::Snow);
        assert_eq!(ground_kind("KorhalCityEx2_1", None), GroundKind::Concrete);
        assert_eq!(ground_kind("ShakurasTemple3", None), GroundKind::Tiles);
        assert_eq!(ground_kind("AgriaFoliage", None), GroundKind::Foliage);
        assert_eq!(ground_kind("cobblestone", None), GroundKind::Cobblestone);
    }

    #[test]
    fn it_reads_the_kind_of_ground_from_the_colors_of_the_unnamed_ones() {
        // Elsecaro3 is green, Voidstorm purple, an unknown gray.
        assert_eq!(
            ground_kind("Elsecaro3", Some([62, 94, 30])),
            GroundKind::Grass
        );
        assert_eq!(
            ground_kind("Voidstorm8", Some([90, 67, 136])),
            GroundKind::Rock
        );
        assert_eq!(ground_kind("Typhon7", Some([66, 61, 48])), GroundKind::Dirt);
        assert_eq!(ground_kind("Zerus7", Some([159, 84, 41])), GroundKind::Sand);
        assert_eq!(ground_kind("Xil2", Some([220, 222, 224])), GroundKind::Snow);
        assert_eq!(ground_kind("Unnamed", None), GroundKind::Dirt);
    }

    #[test]
    fn it_brightens_the_colors_of_the_minimap_together() {
        let tints = layer_tints(
            &[Some([40, 40, 40]), None, Some([20, 60, 20])],
            &[0.5, 0.1, 0.4],
            &[[0.; 3], [0.3, 0.2, 0.1], [0.; 3]],
        );
        // The layer without a color keeps its fallback.
        assert_eq!(tints[1], [0.3, 0.2, 0.1]);
        // The others keep their hue, brightened by the same gain.
        let ratio = tints[0][0] / srgb_to_linear(40);
        assert!(ratio > 1.);
        assert!((tints[2][1] / srgb_to_linear(60) - ratio).abs() < 1e-3);
    }
}
