#version 130

// position is in pixelwindow coordinates
in vec2 frag_position;

uniform vec2 offset;
uniform vec2 spacing;

out vec4 out_color;

void main() {
    vec2 dist = mod(frag_position - offset + 0.5, spacing);
    if((0.0 <= dist.x && dist.x < 1.0) || (0 <= dist.y && dist.y < 1.0)) {
        out_color = vec4(0.0, 0.0, 0.0, 0.1);
    } else {
        out_color = vec4(0.0, 0.0, 0.0, 0.0);
    }
}