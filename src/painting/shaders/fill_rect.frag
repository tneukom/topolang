#version 300 es

uniform sampler2D tile_atlas_texture;
uniform highp float time;

in highp vec2 frag_texcoord;

out highp vec4 out_color;

void main() {
    highp vec2 window_frag_coord = gl_FragCoord.xy;
    highp vec4 texcolor = texture(tile_atlas_texture, frag_texcoord);
    // Wave effect

    highp float u = dot(window_frag_coord, vec2(1.0, 1.0)) / sqrt(2.0);
    highp float v = dot(window_frag_coord, vec2(-1.0, 1.0)) / sqrt(2.0);
    highp vec2 uv = vec2(u, v);

    out_color = vec4(texcolor.a * texcolor.rgb, texcolor.a);
}