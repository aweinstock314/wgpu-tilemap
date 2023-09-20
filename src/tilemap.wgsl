struct Tiledata {
    width: u32,
    height: u32,
    tile_width: u32,
    tile_height: u32,
}

struct Tilemap {
    // transform maps from [0, 1]x[0,1] to world coordinates
    transform: mat4x4<f32>,
    width: u32,
    height: u32,
    noise_data: u32,
    pad: u32,
}

// camera maps from world coordinates to NDC
@group(0) @binding(0) var<uniform> camera: mat4x4<f32>;

@group(1) @binding(0) var<uniform> tiledata: Tiledata;
@group(1) @binding(1) var tilemap_data: texture_2d_array<f32>;

@group(2) @binding(0) var<uniform> tilemap: Tilemap;
@group(2) @binding(1) var tilemap_indices: texture_2d<u32>;

struct TilemapFragData {
    @builtin(position) position: vec4<f32>,
    @location(0) tilepos: vec2<f32>,
    @location(1) pixelpos: vec2<f32>,
}

const QUAD_VERTICES: array<vec4<f32>, 6> = array<vec4<f32>, 6>(
    vec4<f32>(0.0, 0.0, 0.0, 1.0),
    vec4<f32>(1.0, 0.0, 0.0, 1.0),
    vec4<f32>(0.0, 1.0, 0.0, 1.0),
    vec4<f32>(0.0, 1.0, 0.0, 1.0),
    vec4<f32>(1.0, 0.0, 0.0, 1.0),
    vec4<f32>(1.0, 1.0, 0.0, 1.0),
);

@vertex
fn tilemap_vert_main(@builtin(vertex_index) vertex_index: u32) -> TilemapFragData {
    var quad_vertices = QUAD_VERTICES;
    let position = quad_vertices[vertex_index % 6u]; 
    var ret: TilemapFragData;
    ret.position = camera * tilemap.transform * position;
    let uvpos = position.xy;
    let uvflip = vec2(uvpos.x, 1.0 - uvpos.y);
    let size_in_tiles = vec2<f32>(f32(tilemap.width), f32(tilemap.height));
    let size_of_tile = vec2(tiledata.tile_width, tiledata.tile_height);
    let size_in_pixels = size_in_tiles * vec2<f32>(size_of_tile);
    ret.tilepos = uvflip * size_in_tiles;
    ret.pixelpos = uvflip * size_in_pixels;
    return ret;
}

@fragment
fn tilemap_frag_main(data: TilemapFragData) -> @location(0) vec4<f32> {
    var tile: u32 = textureLoad(tilemap_indices, vec2<u32>(data.tilepos), 0).r;
    let size_of_tile = vec2(tiledata.tile_width, tiledata.tile_height);
    let subpos = vec2<u32>(data.pixelpos) % size_of_tile;
    var col: vec4<f32> = textureLoad(tilemap_data, subpos, tile, 0);
    let noise_magnitude = f32(tilemap.noise_data & 0xffffu) / 65536.0;
    if noise_magnitude != 0.0 {
        let noise_res = f32((tilemap.noise_data >> 16u) & 0xffu);
        var noise: vec3<f32> = pcg3d(vec2<f32>(size_of_tile * vec2<u32>(vec2<f32>(noise_res, noise_res) * data.tilepos)));
        col += noise_magnitude * vec4(noise.x, noise.x, noise.x, 0.0);
        col = clamp(vec4(0.0, 0.0, 0.0, 0.0), vec4(1.0, 1.0, 1.0, 1.0), col);
    }
    if col.a == 0.0 {
        discard;
    }
    return col;
}

fn pcg3d(uv: vec2<f32>) -> vec3<f32> {
    var a = bitcast<vec2<u32>>(uv);
    var b = vec3(a.xy, a.x ^ a.y);
    var v: vec3<u32> = b * 1664525u + 1013904223u;

    v.x += v.y*v.z;
    v.y += v.z*v.x;
    v.z += v.x*v.y;

    v ^= vec3(v.x >> 16u, v.y >> 16u, v.z >> 16u);

    v.x += v.y*v.z;
    v.y += v.z*v.x;
    v.z += v.x*v.y;

    return vec3<f32>(vec3<u32>(v.x & 0xffu, v.y & 0xffu, v.z & 0xffu)) / 255.0;
}
