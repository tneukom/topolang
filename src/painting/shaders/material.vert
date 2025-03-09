#version 300 es

in highp vec2 in_world_position;
in highp vec2 in_bitmap_uv;

uniform highp mat3 world_to_device;
uniform highp mat3 bitmap_to_gltexture;

out highp vec2 gltexture_uv;

void main() {
    gltexture_uv = (bitmap_to_gltexture * vec3(in_bitmap_uv, 1.0)).xy;
    highp vec2 device_position = (world_to_device * vec3(in_world_position, 1.0)).xy;
    gl_Position = vec4(device_position, 0.0, 1.0);
}