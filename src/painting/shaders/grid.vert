#version 300 es

in highp vec2 in_world_position;

out highp vec2 world_position;

uniform highp mat3 world_to_device;

void main() {
    world_position = in_world_position;

    highp vec2 device_position = (world_to_device * vec3(world_position, 1.0)).xy;

    gl_Position = vec4(device_position, 0.0, 1.0);
}