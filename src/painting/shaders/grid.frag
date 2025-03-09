#version 300 es

in highp vec2 world_position;

uniform highp float world_spacing;

uniform highp mat3 world_to_view;

out highp vec4 out_color;

void main() {
    highp float world_pixel_size = 1.0 / world_to_view[0][0];
    highp vec2 mod_spacing = mod(world_position, world_spacing);

    if (mod_spacing.x < world_pixel_size || mod_spacing.y < world_pixel_size) {
        out_color = vec4(0.0, 0.0, 0.0, 0.1);
    } else {
        out_color = vec4(0.0, 0.0, 0.0, 0.0);
    }
}