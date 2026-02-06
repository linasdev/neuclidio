#version 450

layout(location = 0) in vec3 passPosition;
layout(location = 1) in vec3 passNormal;
layout(location = 2) in vec2 passTextureCoordinate;

layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(passNormal.r, passNormal.g, passNormal.b, 1.0);
}
