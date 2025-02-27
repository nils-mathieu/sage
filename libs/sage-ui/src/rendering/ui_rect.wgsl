struct VertexInput {
    // Builtins

    @builtin(vertex_index) vertex_index: u32,

    // UiRectInstance

    // @location(0) position: vec2f,
    // @location(1) size: vec2f,
    // @location(2) background_color: vec4f,
    // @location(3) border_color: vec4f,
    // @location(4) outline_color: vec4f,
    // @location(5) border_radius: vec4f,
    // @location(6) border_thickness: vec4f,
    // @location(7) outline_thickness: f32,
    // @location(8) outline_offset: f32,
    // @location(9) flags: u32,
    // @location(10) z_index: i32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,

    @location(0) @interpolate(linear) uv: vec2f,
    @location(1) @interpolate(flat) size: vec2f,
    @location(2) @interpolate(flat) background_color: vec4f,
    @location(3) @interpolate(flat) border_color: vec4f,
    @location(4) @interpolate(flat) outline_color: vec4f,
    @location(5) @interpolate(flat) border_radius: vec4f,
    @location(6) @interpolate(flat) border_thickness: vec4f,
    @location(7) @interpolate(flat) outline_thickness: f32,
    @location(8) @interpolate(flat) outline_offset: f32,
    @location(9) @interpolate(flat) flags: u32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let vertex_pos = vec2f(
        f32(in.vertex_index & 1u),
        f32(in.vertex_index >> 1u)
    );

    var out: VertexOutput;
    out.clip_position = vec4f(vertex_pos, 0.0, 1.0);
    out.uv = vertex_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return vec4f(in.uv, 0.0, 1.0);
}
