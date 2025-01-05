#version 300 es

in highp vec2 in_device_position;

out highp vec2 view_frag_position;

uniform highp mat3 device_to_view;

void main() {
    view_frag_position = (device_to_view * vec3(in_device_position, 1.0)).xy;
    gl_Position = vec4(in_device_position, 0.0, 1.0);
}