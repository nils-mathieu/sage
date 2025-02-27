struct View {
    resolution: vec2u,
}

@group(0) @binding(0)
var<uniform> view: View;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec2i,
    @location(1) size: vec2u,
    @location(2) atlas_position: vec2u,
    @location(3) color: vec4f,
    @location(4) flags: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @interpolate(flat) @location(0) color: vec4f,
    @interpolate(linear) @location(1) uv: vec2f,
    @interpolate(flat) @location(2) flags: u32,
}

@group(1) @binding(0) var atlas_sampler: sampler;
@group(1) @binding(1) var color_atlas: texture_2d<f32>;
@group(1) @binding(2) var mask_atlas: texture_2d<f32>;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let vertex_pos = vec2u(in.vertex_index & 1u, in.vertex_index >> 1u);

    var dim: vec2u;
    switch (in.flags & 1u) {
        case 0u: { dim = textureDimensions(color_atlas); }
        case 1u: { dim = textureDimensions(mask_atlas); }
        default: {}
    }

    let corner_offset = in.size * vertex_pos;
    let screen_pos = in.position + vec2i(corner_offset);
    let clip_pos = vec2f(1.0, -1.0) * (2.0 * vec2f(screen_pos) / vec2f(view.resolution) - 1.0);
    let uv = vec2f(in.atlas_position + corner_offset) / vec2f(dim);

    var out: VertexOutput;
    out.clip_position = vec4f(clip_pos, 0.0, 1.0);
    out.color = in.color;
    out.uv = uv;
    out.flags = in.flags;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    switch (in.flags & 1u) {
        case 0u: { return textureSample(color_atlas, atlas_sampler, in.uv); }
        case 1u: { return vec4f(in.color.rgb, in.color.a * textureSample(mask_atlas, atlas_sampler, in.uv).r); }
        default: { return vec4f(1.0, 0.0, 0.0, 1.0); }
    }
}
