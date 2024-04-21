
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) metadata: i32,
    @location(3) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) metadata: i32,
    @location(2) color: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.metadata = in.metadata;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var is_inverse: bool = (in.metadata & 1) > 0;
    var is_curve: bool = (in.metadata & 2) > 0;
    var fill: f32 = 0.0;
    if !is_curve {
        fill = 1.0;
    } else if is_inverse {
        if in.uv.y < in.uv.x*in.uv.x {
            fill = 1.0;
        }
    } else {
        if in.uv.y >= in.uv.x*in.uv.x {
                    fill = 1.0;
                }
    }
    return vec4(in.color, fill);
}