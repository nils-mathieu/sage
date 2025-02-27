struct View {
    resolution: vec2u,
}

@group(0) @binding(0)
var<uniform> view: View;

struct VertexInput {
    // Builtins

    @builtin(vertex_index) vertex_index: u32,

    // UiRectInstance

    @location(0) position: vec2i,
    @location(1) size: vec2u,
    @location(2) corner_radius: vec4f,
    @location(3) border_thickness: f32,
    @location(4) color: vec4f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,

    @location(0) @interpolate(linear) uv: vec2f,

    @location(1) @interpolate(linear) point: vec2f, // Relative to the rectangle's center.
    @location(2) @interpolate(flat) size: vec2f,
    @location(3) @interpolate(flat) border_thickness: f32,
    @location(4) @interpolate(flat) color: vec4f,
    @location(5) @interpolate(flat) corner_radius: vec4f,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let vertex_pos = vec2u(in.vertex_index & 1u, in.vertex_index >> 1u);

    // This is still upside-down because Y points up in WGSL.
    let clip = vec2f(in.position + vec2i(vertex_pos * in.size)) / vec2f(view.resolution) * 2.0 - 1.0;

    var out: VertexOutput;
    out.clip_position = vec4f(clip.x, -clip.y, 0.0, 1.0);
    out.uv = vec2f(vertex_pos);
    out.point = vec2f(vertex_pos * in.size) - vec2f(in.size) * 0.5;
    out.size = vec2f(in.size);
    out.border_thickness = in.border_thickness;
    out.color = in.color;
    out.corner_radius = in.corner_radius;
    return out;
}

// Computes the signed distance from a point to a rounded rectangle's border.
//
// # Parameters
//
// * `point`: The point to compute the distance to.
//
// * `size`: The size of the rounded rectangle.
//
// * `radii`: The radius of each rounded corner, in the order: top-left, top-right, bottom-right,
//   bottom-left.
//
// Note that the rectangle is assumed to be centered at the origin.
//
// # Returns
//
// This function returns the signed distance from `point` to the rectangle's closest bound.
// Positive values mean that the point is outside the rectangle, while negative values mean that
// it is inside it. Zero means that the point is on the rectangle's bound.
fn sd_rounded_rect(point: vec2f, size: vec2f, radii: vec4f) -> f32 {
    // Select which radius we're going to use based on the point's position. This effectively
    // mirrors the point to the first quadrant.
    let rs = select(radii.xy, radii.wz, point.y > 0.0);
    let radius = select(rs.x, rs.y, point.x > 0.0);

    // Vector from closest corner to `point`.
    let corner_to_point = abs(point) - size * 0.5;
    // Vector from the closest radius circle to `point`.
    let q = corner_to_point + radius;

    let l = length(max(q, vec2f(0.0, 0.0)));
    let m = min(max(q.x, q.y), 0.0);
    return l + m - radius;
}

fn sd_inset_rounded_rect(point: vec2f, size: vec2f, radii: vec4f, inset: vec4f) -> f32 {
    // Compute the metrics of the inner rectangle.
    let inner_size = size - inset.xy - inset.zw;
    let inner_center = inset.xy + 0.5 * inner_size - 0.5 * size;
    let inner_point = point - inner_center;

    var r = radii;
    r.x = r.x - max(inset.x, inset.y);
    r.y = r.y - max(inset.z, inset.y);
    r.z = r.z - max(inset.z, inset.w);
    r.w = r.w - max(inset.x, inset.w);

    // Clamp the computed radius.
    let half_size = inner_size * 0.5;
    let min_size = min(half_size.x, half_size.y);
    r = min(max(r, vec4f(0.0)), vec4<f32>(min_size));

    return sd_rounded_rect(inner_point, inner_size, r);
}

// Draws the rectangle as a border.
fn rounded_border(in: VertexOutput) -> f32 {
    let outer = sd_rounded_rect(in.point, in.size, in.corner_radius);
    let inner = sd_inset_rounded_rect(in.point, in.size, in.corner_radius, vec4f(in.border_thickness));
    let dist = max(outer, -inner);
    return saturate(0.5 - dist);
}

// Draw the rectangle as a filled shape.
fn rounded_rect(in: VertexOutput) -> f32 {
    let sd = sd_rounded_rect(in.point, in.size, in.corner_radius);
    return saturate(0.5 - sd);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    var t = 0.0;
    if (in.border_thickness == 0.0) {
        t = rounded_rect(in);
    } else {
        t = rounded_border(in);
    }
    return vec4f(in.color.rgb, in.color.a * t);
}
