#version 300 es
precision highp float;

in vec2 in_world_position;
in vec2 in_bitmap_uv;

uniform mat3 device_from_world;
uniform mat3 gltexture_from_bitmap;

out vec2 pass_gltexture_uv;
out vec2 pass_world_position;

void main() {
    pass_gltexture_uv = (gltexture_from_bitmap * vec3(in_bitmap_uv, 1.0)).xy;
    pass_world_position = in_world_position;

    vec2 device_position = (device_from_world * vec3(in_world_position, 1.0)).xy;
    gl_Position = vec4(device_position, 0.0, 1.0);
}