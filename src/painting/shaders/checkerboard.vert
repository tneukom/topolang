#version 300 es
precision highp float;

in vec2 in_world_position;

out vec2 world_position;

uniform mat3 device_from_world;

void main() {
    world_position = in_world_position;

    vec2 device_position = (device_from_world * vec3(world_position, 1.0)).xy;

    gl_Position = vec4(device_position, 0.0, 1.0);
}