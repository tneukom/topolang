#version 300 es

in highp vec2 in_device_position;

void main() {
    gl_Position = vec4(in_device_position, 0.0, 1.0);
}