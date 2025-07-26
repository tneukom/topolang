#version 130

in vec2 pass_view_position;
in vec2 pass_view_line_start;
in vec2 pass_view_line_end;
in float pass_line_width;
in float pass_arc_length;

uniform float time;

out vec4 out_color;

// Calculate the coordinates of p in the reference frame with origin at a, b1 = perp line direction, b2 = line direction
vec2 line_st(vec2 p, vec2 a, vec2 b) {
    // Let d = (b - a)/|b - a| and p = a + s*d + t*perp(d)
    // <p, d> = <a, d> + s<d, d> therefore s = <p - a, d>/<d, d> = <p - a, d>
    // <p, perp(d)> = <a, perp(d)> + t<perp(d), perp(d)> therefore t = <p - a, perp(d)>
    vec2 dir = normalize(b - a);
    vec2 perp_dir = vec2(-dir.y, dir.x);
    float t = dot(p - a, dir);// along line direction
    float s = dot(p - a, perp_dir);

    return vec2(s, t);
}

// Returns value between 0 and 1 where 1 is a dash.
float dashes(float t) {
    t = mod(t, 10.0);
    // Smoothstep with a gap < 2 doesn't animate smoothly
    if(t < 5.0) {
        return smoothstep(1.5, 3.5, t);
    } else {
        return 1.0 - smoothstep(6.5, 8.5, t);
    }
}

void main() {
    vec2 line_p = line_st(pass_view_position, pass_view_line_start, pass_view_line_end);

    // distance to line
    float line_length = distance(pass_view_line_start, pass_view_line_end);
    float closest_t = clamp(line_p.t, 0.0, line_length);
    float dist_t = abs(line_p.t - closest_t);
    float dist_to_line_segment = sqrt(dist_t * dist_t + line_p.s * line_p.s);

    if(dist_to_line_segment < pass_line_width) {
        float arc_length = pass_arc_length + line_p.t;
        vec4 dash_color = vec4(1.0, 0.8, 0.0, 1.0);
        vec4 blank_color = vec4(0.0, 0.0, 0.0, 1.0);
        out_color = mix(dash_color, blank_color, dashes(arc_length + 4.0 * time));
        gl_FragDepth = dist_to_line_segment / pass_line_width;
    } else {
        discard;
    }
}