#version 450

layout(location = 0) out vec2 passTextureCoordinate;

void main() {
    // Generate texture coordinates (0,0), (2,0), (0,2)
    passTextureCoordinate = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);

    // Map texture coordinates to Clip Space coordinates:
    // (0,0) -> (-1.0, -1.0)
    // (2,0) -> ( 3.0, -1.0)
    // (0,2) -> (-1.0,  3.0)
    gl_Position = vec4(passTextureCoordinate * 2.0f - 1.0f, 0.0f, 1.0f);
}
