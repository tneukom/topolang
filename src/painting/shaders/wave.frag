#version 130

in vec2 pass_view_position;
in vec2 pass_view_line_start;
in vec2 pass_view_line_end;

uniform float wave_radius;
uniform float line_width;

out vec4 out_color;

// Returns (s, t) such that p = ray_start + s * perp(ray_dir) + t * ray_dir
// ray_dir should have length 1
vec2 ray_st(vec2 p, vec2 ray_start, vec2 ray_dir) {
    // <p, ray_dir> = <ray_start, ray_dir> + s<ray_dir, ray_dir> therefore
    // s = <p - ray_start, ray_dir>
    // <p, perp(ray_dir)> = <ray_start, perp(d)> + t<perp(ray_dir), perp(ray_dir)> therefore
    // t = <p - ray_start, perp(ray_dir)>
    vec2 perp_ray_dir = vec2(-ray_dir.y, ray_dir.x);
    float t = dot(p - ray_start, ray_dir); // along line direction
    float s = dot(p - ray_start, perp_ray_dir);

    return vec2(s, t);
}

void main() {
    vec2 line_delta = pass_view_line_end - pass_view_line_start;
    float line_length = length(line_delta);
    vec2 ray_dir = line_delta / line_length;
    vec2 st = ray_st(pass_view_position, pass_view_line_start, ray_dir);

    // distance to line
    float closest_t = clamp(st.t, 0.0, line_length);
    float dist_t = abs(st.t - closest_t);
    float dist_to_line_segment = sqrt(dist_t * dist_t + st.s * st.s);

    gl_FragDepth = dist_to_line_segment / 20.0;
    //out_color = vec4(dist_to_line_segment / 20.0, 0.0, 0.0, 1.0);
    if(abs(dist_to_line_segment - wave_radius) < 1.0) {
        out_color = vec4(1.0, dist_to_line_segment / 20.0, 0.0, 1.0);
    } else {
        out_color = vec4(0.0, dist_to_line_segment / 20.0, 0.0, 1.0);
    }
}