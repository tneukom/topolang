#version 300 es

in highp vec2 in_device_position;

out highp vec2 window_frag_position;

uniform highp mat3 device_to_window;

void main() {
    window_frag_position = (device_to_window * vec3(in_device_position, 1.0)).xy;
    gl_Position = vec4(in_device_position, 0.0, 1.0);
}