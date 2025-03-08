#version 300 es

in highp vec2 world_position;

uniform highp float size;

out highp vec4 out_color;

void main() {
    highp vec2 scaled = (1.0 / size) * world_position;
    highp float checker = mod(floor(scaled.x) + floor(scaled.y), 2.0);
    if (checker < 1.0) {
        out_color = vec4(0.6, 0.6, 0.6, 1.0);
    } else {
        out_color = vec4(0.7, 0.7, 0.7, 1.0);
    }
}