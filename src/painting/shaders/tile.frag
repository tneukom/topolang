#version 300 es

uniform sampler2D tile_atlas_texture;
uniform highp float time;

in highp vec2 frag_texcoord;

out highp vec4 out_color;

void main() {
    highp vec2 window_frag_coord = gl_FragCoord.xy;
    highp vec4 texcolor = texture(tile_atlas_texture, frag_texcoord);
    // Wave effect
    const highp float PI = 3.141592653589;

    highp float u = dot(window_frag_coord, vec2(1.0, 1.0)) / sqrt(2.0);
    highp float v = dot(window_frag_coord, vec2(-1.0, 1.0)) / sqrt(2.0);
    highp vec2 uv = vec2(u, v);

    if(texcolor.a == 180.0/255.0) {
        // Rule frame and arrow
        highp float s = u + 4.0 * sin(1.0/25.0 * PI * v) + 4.0 * time;
        //highp float s = 1.0/10.0 * PI * (window_frag_coord.x + window_frag_coord.y);
        highp float c = mod(s, 16.0) > 8.0 ? 1.0 : 0.6;
        out_color = vec4(c * texcolor.a * texcolor.rgb, 1.0);
    } else if(texcolor.a == 170.0/255.0) {
        // Rigid match
        highp vec2 i = floor(uv / 16.0);
        highp float checkers = mod(i.x + i.y, 2.0) > 0.0 ? 1.0 : 0.6;
        out_color = vec4(checkers * texcolor.a * texcolor.rgb, 1.0);
    } else {
        out_color = vec4(texcolor.a * texcolor.rgb, texcolor.a);
    }

}