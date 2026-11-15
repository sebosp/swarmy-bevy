// The terrain painted with its 8 texture layers: the weights of the layers at each point of the
// map (t3TextureMasks), each layer a tint (its color in the minimap) times the detail of a free
// texture of the same kind of ground. Lit by a fixed sun, the cliffs (the sides of the cells) are
// darker.

import bevy_pbr::render::forward_io::VertexOutput;

struct TerrainSplat {
    // The linear tint of each layer.
    tints: array<vec4<f32>, 8>,
    // x: the world units of a map cell, y: the cells across the masks, z: the cells along them,
    // w: the cells a texture covers before it repeats.
    map: vec4<f32>,
    // xyz: the direction towards the sun, w: the light of the sides facing away from it.
    sun: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> splat: TerrainSplat;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var weights_low: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var weights_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var weights_high: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var weights_high_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var details: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var details_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // The cells are drawn at (row, height, column): x is the map's y, z its x.
    let cell = vec2<f32>(in.world_position.z, in.world_position.x) / splat.map.x;
    let uv = cell / splat.map.yz;
    let low = textureSample(weights_low, weights_sampler, uv);
    let high = textureSample(weights_high, weights_high_sampler, uv);
    var weights = array<f32, 8>(low.r, low.g, low.b, low.a, high.r, high.g, high.b, high.a);
    let normal = normalize(in.world_normal);
    // The tops take the texture from above; the sides (cliffs) from the side, along the side and
    // up, else they are streaks that shimmer when the camera moves.
    var detail_uv = cell / splat.map.w;
    if (abs(normal.y) < 0.5) {
        let along = select(in.world_position.x, in.world_position.z, abs(normal.x) > 0.5);
        detail_uv = vec2<f32>(along, in.world_position.y) / splat.map.x / splat.map.w;
    }
    var color = vec3<f32>(0.0);
    var total = 0.0;
    for (var layer = 0; layer < 8; layer++) {
        let detail = textureSample(details, details_sampler, detail_uv, layer).rgb * 2.0;
        color += weights[layer] * splat.tints[layer].rgb * detail;
        total += weights[layer];
    }
    color = color / max(total, 0.001);
    let light = splat.sun.w + (1.0 - splat.sun.w) * max(dot(normal, normalize(splat.sun.xyz)), 0.0);
    // The cliffs: the sides of the cells, darker, as the rock under the ground.
    let cliff = mix(0.55, 1.0, smoothstep(0.3, 0.8, abs(normal.y)));
    return vec4<f32>(color * light * cliff, 1.0);
}
