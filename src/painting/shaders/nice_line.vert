#version 300 es
precision highp float;

in vec2 in_line_start;
in vec2 in_line_end;
in float in_line_width;
in float in_arc_length;
in int in_quad_corner;

out vec2 pass_view_position;
out vec2 pass_view_line_start;
out vec2 pass_view_line_end;
out float pass_line_width;
out float pass_arc_length;

uniform mat3 view_from;
uniform mat3 device_from_view;

void main() {
    pass_view_line_start = (view_from * vec3(in_line_start, 1.0)).xy;
    pass_view_line_end = (view_from * vec3(in_line_end, 1.0)).xy;
    pass_line_width = in_line_width;

    // Assuming uniform scaling
    float scale_view_from = view_from[0][0];
    pass_arc_length = scale_view_from * in_arc_length;

    vec2 d = normalize(pass_view_line_end - pass_view_line_start);
    // orthogonal_ccw
    vec2 d_perp = vec2(-d.y, d.x);

    if(in_quad_corner == 0) {
        pass_view_position = pass_view_line_start + in_line_width * (-d - d_perp);
    } else if(in_quad_corner == 1) {
        pass_view_position = pass_view_line_start + in_line_width * (-d + d_perp);
    } else if(in_quad_corner == 2) {
        pass_view_position = pass_view_line_end + in_line_width * (d + d_perp);
    } else {
        pass_view_position = pass_view_line_end + in_line_width * (d - d_perp);
    }

    vec2 device_position = (device_from_view * vec3(pass_view_position, 1.0)).xy;
    gl_Position = vec4(device_position, 0.0, 1.0);
}