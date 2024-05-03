
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) metadata: i32,
    @location(3) color_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) metadata: i32,
    @location(2) color_index: u32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.metadata = in.metadata;
    out.color_index = in.color_index;
    return out;
}

@group(0) @binding(0)
var<storage> color: array<vec4<f32>>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var is_inverse: bool = (in.metadata & 1) > 0;
    var is_curve: bool = (in.metadata & 2) > 0;
    var c: vec4<f32> = color[in.color_index];
    var curve_alpha: f32 = sample_curve(is_inverse, is_curve, in.uv.xy);

    return vec4(c.xyz, c.w * curve_alpha);
}

fn sample_curve(is_inverse: bool, is_curve: bool, uv: vec2<f32>) -> f32 {
    return 1.0 - f32(is_curve & ((is_inverse & (uv.y < uv.x*uv.x)) | (!is_inverse & (uv.y >= uv.x*uv.x))));
}