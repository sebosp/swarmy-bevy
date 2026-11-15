//! The terrain textures from Poly Haven (polyhaven.com, CC0): downloaded once through its API into
//! the cache directory, with a user agent that names swarmy-bevy as its terms ask.

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use std::path::PathBuf;

/// The side of the textures on the GPU, in pixels.
pub const DETAIL_SIZE: usize = 512;

const USER_AGENT: &str = concat!(
    "swarmy-bevy/",
    env!("CARGO_PKG_VERSION"),
    " (terrain textures; github.com/sebosp/swarmy-bevy)"
);

/// The directory of the downloaded textures, None without a home directory.
pub fn cache_dir() -> Option<PathBuf> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(cache.join("swarmy-bevy").join("polyhaven"))
}

/// The color texture (1k JPG) of a Poly Haven texture, from the cache, else downloaded into it.
pub fn diffuse_jpg(id: &str) -> Result<Vec<u8>, String> {
    let dir = cache_dir().ok_or("no cache directory")?;
    let path = dir.join(format!("{id}_diff_1k.jpg"));
    if let Ok(bytes) = std::fs::read(&path) {
        return Ok(bytes);
    }
    let files: serde_json::Value = serde_json::from_str(
        &ureq::get(format!("https://api.polyhaven.com/files/{id}"))
            .header("User-Agent", USER_AGENT)
            .call()
            .map_err(|err| format!("{id}: {err}"))?
            .body_mut()
            .read_to_string()
            .map_err(|err| format!("{id}: {err}"))?,
    )
    .map_err(|err| format!("{id}: {err}"))?;
    let url = files["Diffuse"]["1k"]["jpg"]["url"]
        .as_str()
        .ok_or_else(|| format!("{id}: no 1k JPG"))?;
    let bytes = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| format!("{id}: {err}"))?
        .body_mut()
        .with_config()
        .limit(32 * 1024 * 1024)
        .read_to_vec()
        .map_err(|err| format!("{id}: {err}"))?;
    std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    // Written whole or not at all: a half file would stay broken in the cache.
    let partial = dir.join(format!("{id}_diff_1k.jpg.part"));
    std::fs::write(&partial, &bytes).map_err(|err| err.to_string())?;
    std::fs::rename(&partial, &path).map_err(|err| err.to_string())?;
    Ok(bytes)
}

/// A texture as a layer of the terrain: [`DETAIL_SIZE`] pixels square, linear, each channel divided
/// by its mean and halved (the mean is 0.5), so that the shader multiplies the color of the layer
/// by its detail; and the mean linear color of the texture.
#[derive(Debug, Clone)]
pub struct DetailLayer {
    pub pixels: Vec<u8>,
    pub mean: [f32; 3],
}

impl DetailLayer {
    /// A layer without detail, for the textures that could not be downloaded.
    pub fn flat(mean: [f32; 3]) -> Self {
        Self {
            pixels: [128, 128, 128, 255].repeat(DETAIL_SIZE * DETAIL_SIZE),
            mean,
        }
    }

    /// The layer of a JPG or PNG texture.
    pub fn from_image_bytes(bytes: &[u8], extension: &str) -> Result<Self, String> {
        let image = Image::from_buffer(
            bytes,
            ImageType::Extension(extension),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .map_err(|err| err.to_string())?
        .convert(bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb)
        .ok_or("not an RGBA image")?;
        let (width, height) = (image.width() as usize, image.height() as usize);
        let data = image.data.as_ref().ok_or("no pixels")?;
        Ok(Self::from_srgba(data, width, height))
    }

    /// The layer of sRGB RGBA8 pixels, resized to [`DETAIL_SIZE`] by averaging.
    pub fn from_srgba(data: &[u8], width: usize, height: usize) -> Self {
        let lut: Vec<f32> = (0..=255u8).map(super::materials::srgb_to_linear).collect();
        let mut linear = vec![[0f32; 3]; DETAIL_SIZE * DETAIL_SIZE];
        let mut sum = [0f64; 3];
        for (y, row) in linear.chunks_mut(DETAIL_SIZE).enumerate() {
            let (y0, y1) = (
                y * height / DETAIL_SIZE,
                ((y + 1) * height / DETAIL_SIZE).max(y * height / DETAIL_SIZE + 1),
            );
            for (x, pixel) in row.iter_mut().enumerate() {
                let (x0, x1) = (
                    x * width / DETAIL_SIZE,
                    ((x + 1) * width / DETAIL_SIZE).max(x * width / DETAIL_SIZE + 1),
                );
                let mut acc = [0f32; 3];
                for sy in y0..y1.min(height) {
                    for sx in x0..x1.min(width) {
                        let start = (sy * width + sx) * 4;
                        for c in 0..3 {
                            acc[c] += lut[data[start + c] as usize];
                        }
                    }
                }
                let count = ((y1.min(height) - y0) * (x1.min(width) - x0)).max(1) as f32;
                for c in 0..3 {
                    pixel[c] = acc[c] / count;
                    sum[c] += pixel[c] as f64;
                }
            }
        }
        let count = (DETAIL_SIZE * DETAIL_SIZE) as f64;
        let mean = [
            (sum[0] / count) as f32,
            (sum[1] / count) as f32,
            (sum[2] / count) as f32,
        ];
        let mut pixels = Vec::with_capacity(DETAIL_SIZE * DETAIL_SIZE * 4);
        for pixel in &linear {
            for c in 0..3 {
                let ratio = if mean[c] > 1e-4 {
                    pixel[c] / mean[c]
                } else {
                    1.
                };
                pixels.push(((ratio * 0.5).clamp(0., 1.) * 255.).round() as u8);
            }
            pixels.push(255);
        }
        Self { pixels, mean }
    }
}

/// The mip levels of a square RGBA8 image, the image first and each next one half its side,
/// averaged, down to 1 pixel: the textures far from the camera do not shimmer.
pub fn mip_chain(pixels: &[u8], size: usize) -> (Vec<u8>, u32) {
    let mut out = pixels.to_vec();
    let mut level = pixels.to_vec();
    let mut side = size;
    let mut levels = 1;
    while side > 1 {
        let half = side / 2;
        let mut next = vec![0u8; half * half * 4];
        for y in 0..half {
            for x in 0..half {
                for c in 0..4 {
                    let at = |dx: usize, dy: usize| {
                        level[((y * 2 + dy) * side + x * 2 + dx) * 4 + c] as u32
                    };
                    next[(y * half + x) * 4 + c] =
                        ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8;
                }
            }
        }
        out.extend_from_slice(&next);
        level = next;
        side = half;
        levels += 1;
    }
    (out, levels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_divides_the_texture_by_its_mean() {
        // Half of the pixels at 200, half at 100 (sRGB): the mean is between, the bright half
        // above 128, the dark half below.
        let (w, h) = (64, 64);
        let mut data = vec![];
        for y in 0..h {
            for _ in 0..w {
                let v = if y < h / 2 { 200 } else { 100 };
                data.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let layer = DetailLayer::from_srgba(&data, w, h);
        assert_eq!(layer.pixels.len(), DETAIL_SIZE * DETAIL_SIZE * 4);
        assert!(layer.pixels[0] > 128);
        assert!(layer.pixels[(DETAIL_SIZE * DETAIL_SIZE - 1) * 4] < 128);
        let expected_mean = (srgb(200) + srgb(100)) / 2.;
        assert!((layer.mean[0] - expected_mean).abs() < 1e-3);
    }

    fn srgb(v: u8) -> f32 {
        super::super::materials::srgb_to_linear(v)
    }

    #[test]
    fn it_halves_the_mip_levels_down_to_one_pixel() {
        let (data, levels) = mip_chain(&[10, 20, 30, 255].repeat(16), 4);
        assert_eq!(levels, 3);
        assert_eq!(data.len(), (16 + 4 + 1) * 4);
        assert_eq!(&data[data.len() - 4..], &[10, 20, 30, 255]);
    }
}
