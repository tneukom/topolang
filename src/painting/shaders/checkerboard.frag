#version 300 es
precision highp float;

in vec2 world_position;

uniform float size;

// SRGB color space
uniform vec4 even_srgba;
uniform vec4 odd_srgba;

out vec4 out_color;

void main() {
    vec2 scaled = (1.0 / size) * world_position;
    float checker = mod(floor(scaled.x) + floor(scaled.y), 2.0);
    if (checker < 1.0) {
        out_color = even_srgba;
    } else {
        out_color = odd_srgba;
    }
}