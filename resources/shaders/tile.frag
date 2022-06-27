#version 130

uniform sampler2D tile_atlas_texture;

in vec2 frag_texcoord;

out vec4 out_color;

void main() {
    vec4 texcolor = texture(tile_atlas_texture, frag_texcoord);
    out_color = vec4(texcolor.a * texcolor.rgb, texcolor.a);
}