#version 300 es
precision highp float;

in vec2 in_world_position;
in vec2 in_bitmap_uv;

uniform mat3 world_to_device;
uniform mat3 bitmap_to_gltexture;

out vec2 gltexture_uv;

void main() {
    gltexture_uv = (bitmap_to_gltexture * vec3(in_bitmap_uv, 1.0)).xy;
    vec2 device_position = (world_to_device * vec3(in_world_position, 1.0)).xy;
    gl_Position = vec4(device_position, 0.0, 1.0);
}