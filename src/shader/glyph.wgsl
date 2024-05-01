
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
    @location(3) subpixel: vec3<f32>,
}

struct SubpixelOffset {
    @location(4) offset: f32,
}

const SUBPIXEL_VEC: vec3<f32> = vec3<f32>(1.0, 0.0, 0.0);

@vertex
fn vs_main(in: VertexInput, subpixel_offset: SubpixelOffset) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position + vec3<f32>(subpixel_offset.offset, 0.0, 0.0), 1.0);
    out.uv = in.uv;
    out.metadata = in.metadata;
    out.color_index = in.color_index;
    if subpixel_offset.offset < 0.0 {
        out.subpixel = vec3<f32>(0.0, 0.0, 1.0);
    } else if subpixel_offset.offset > 0.0 {
        out.subpixel = vec3<f32>(1.0, 0.0, 0.0);
    } else {
        out.subpixel = vec3<f32>(1.0, 1.0, 1.0);
    }
    return out;
}

@group(0) @binding(0)
var<storage> color: array<vec4<f32>>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var is_inverse: bool = (in.metadata & 1) > 0;
    var is_curve: bool = (in.metadata & 2) > 0;
    var c: vec4<f32> = color[in.color_index];

    return vec4((1.0 - (1.0 - c.xyz) * in.subpixel), c.w * sample_curve(is_inverse, is_curve, in.uv.xy) * (0.6 + in.subpixel.y * 0.4));
}

fn sample_curve(is_inverse: bool, is_curve: bool, uv: vec2<f32>) -> f32 {
    var fill: f32 = 0.0;
    if !is_curve {
        fill = 1.0;
    } else if is_inverse {
        if uv.y < uv.x*uv.x {
            fill = 1.0;
        }
    } else {
        if uv.y >= uv.x*uv.x {
            fill = 1.0;
        }
    }
    return fill;
}