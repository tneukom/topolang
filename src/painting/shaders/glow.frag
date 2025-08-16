#version 300 es
precision highp float;

uniform sampler2D alpha_texture;
uniform float glow_alpha;

in vec2 pass_gltexture_uv;
in vec2 pass_world_position;

out vec4 out_color;


void main() {
    float texel_alpha = texture(alpha_texture, pass_gltexture_uv).r;
    out_color = vec4(1.0, 1.0, 0.0, glow_alpha * texel_alpha);
    //out_color = vec4(1.0, 1.0, 0.0, glow_alpha);
}