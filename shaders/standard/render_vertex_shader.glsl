#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 textureCoordinate;

layout(location = 0) out vec3 passPosition;
layout(location = 1) out vec3 passNormal;
layout(location = 2) out vec2 passTextureCoordinate;

layout(binding = 0) uniform ViewProjectionUniform {
    mat4 view;
    mat4 projection;
} viewProjectionUniform;

layout(push_constant) uniform ModelPushConstant {
    mat4 model;
} modelPushConstant;

void main() {
    vec4 worldSpacePosition = modelPushConstant.model * vec4(position, 1.0f);
    vec4 worldSpaceNormal = modelPushConstant.model * vec4(normal, 0.0f);

    vec4 cameraSpacePosition = viewProjectionUniform.view * worldSpacePosition;
    vec4 screenSpacePosition = viewProjectionUniform.projection * cameraSpacePosition;

    passPosition = worldSpacePosition.xyz;
    passNormal = worldSpaceNormal.xyz;
    passTextureCoordinate = textureCoordinate;

    gl_Position = screenSpacePosition;
}
