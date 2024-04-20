#version 450

layout(location=0) in vec3 position;
layout(location=1) in vec2 uv;
layout(location=2) in int metadata;
layout(location=3) in vec3 color;

layout(location=0) out vec2 uvOut;
layout(location=1) out int metadataOut;
layout(location=2) out vec3 colorOut;

void main() {
    uvOut = uv;
    metadataOut = metadata;
    colorOut = color;
    gl_Position = vec4(position, 1.0);
}