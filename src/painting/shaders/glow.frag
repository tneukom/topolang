#version 300 es
precision highp float;

uniform usampler2D id_texture;
uniform sampler2D alpha_texture;
uniform float glow_alpha;
uniform uint glow_id;

in vec2 pass_gltexture_uv;
in vec2 pass_world_position;

out vec4 out_color;


void main() {
    uint texel_glow_id = texture(id_texture, pass_gltexture_uv).r;

    if(texel_glow_id != glow_id) {
        discard;
    } else {
        float texel_alpha = texture(alpha_texture, pass_gltexture_uv).r;
        out_color = vec4(1.0, 1.0, 0.0, glow_alpha * texel_alpha);
    }
}