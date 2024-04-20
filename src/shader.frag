#version 450

layout(location=0) out vec4 f_color;
layout(location=0) in vec2 uv;
layout(location=1) flat in int metadata;
layout(location=2) in vec3 color;

void main() {
    bool is_inverse = (metadata & 1) > 0;
    bool is_curve = (metadata & 2) > 0;
    bool fill = !is_curve ? true : (is_inverse ? uv.y < uv.x*uv.x : uv.y > uv.x*uv.x);
    f_color = vec4(color, fill);
}