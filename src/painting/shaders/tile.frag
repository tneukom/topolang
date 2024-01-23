#version 300 es

uniform sampler2D tile_atlas_texture;

in highp vec2 frag_texcoord;

out highp vec4 out_color;

void main() {
    highp vec4 texcolor = texture(tile_atlas_texture, frag_texcoord);
    out_color = vec4(texcolor.a * texcolor.rgb, texcolor.a);
}