#version 450
layout(location = 0) in vec2 passTextureCoordinate;

layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D offscreenTexture;

void main() {
    vec3 hdrColor = texture(offscreenTexture, passTextureCoordinate).rgb;
    outColor = vec4(hdrColor, 1.0);
}
