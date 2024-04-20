#version 450

layout(location=0) out vec4 f_color;
layout(location=0) in vec2 uv;

layout(binding=1) uniform sampler2D mysampler;

void main() {
    f_color = texture(mysampler, uv);
}