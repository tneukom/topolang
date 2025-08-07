#version 300 es
precision highp float;

in vec2 in_line_start;
in vec2 in_line_end;
in int in_quad_corner;

out vec2 pass_view_position;
out vec2 pass_view_line_start;
out vec2 pass_view_line_end;

uniform mat3 view_from;
uniform mat3 device_from_view;
uniform float line_width;

void main() {
    pass_view_line_start = (view_from * vec3(in_line_start, 1.0)).xy;
    pass_view_line_end = (view_from * vec3(in_line_end, 1.0)).xy;

    vec2 d = normalize(pass_view_line_end - pass_view_line_start);
    // orthogonal_ccw
    vec2 d_perp = vec2(-d.y, d.x);

    if(in_quad_corner == 0) {
        pass_view_position = pass_view_line_start + line_width * (-d - d_perp);
    } else if(in_quad_corner == 1) {
        pass_view_position = pass_view_line_start + line_width * (-d + d_perp);
    } else if(in_quad_corner == 2) {
        pass_view_position = pass_view_line_end + line_width * (d + d_perp);
    } else {
        pass_view_position = pass_view_line_end + line_width * (d - d_perp);
    }

    vec2 device_position = (device_from_view * vec3(pass_view_position, 1.0)).xy;
    gl_Position = vec4(device_position, 0.0, 1.0);
}