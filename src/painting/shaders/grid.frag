#version 300 es

in highp vec2 window_frag_position;

uniform highp vec2 offset;
uniform highp vec2 spacing;

out highp vec4 out_color;

void main() {
    highp vec2 dist = mod(window_frag_position - offset + 0.5, spacing);
    if((0.0 <= dist.x && dist.x < 1.0) || (0.0 <= dist.y && dist.y < 1.0)) {
        out_color = vec4(0.0, 0.0, 0.0, 0.1);
    } else {
        out_color = vec4(0.0, 0.0, 0.0, 0.0);
    }
}