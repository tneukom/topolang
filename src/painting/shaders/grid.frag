#version 300 es
precision highp float;

in vec2 world_position;

uniform float world_spacing;
uniform mat3 world_to_view;

out vec4 out_color;

void main() {
    float world_pixel_size = 1.0 / world_to_view[0][0];
    vec2 mod_spacing = mod(world_position, world_spacing);

    if (mod_spacing.x < world_pixel_size || mod_spacing.y < world_pixel_size) {
        out_color = vec4(0.0, 0.0, 0.0, 0.1);
    } else {
        out_color = vec4(0.0, 0.0, 0.0, 0.0);
    }
}