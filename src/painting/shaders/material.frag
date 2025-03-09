#version 300 es

uniform sampler2D material_texture;

in highp vec2 gltexture_uv;

out highp vec4 out_color;

void main() {
    highp vec4 color = texture(material_texture, gltexture_uv);
    out_color = vec4(color.a * color.rgb, color.a);
}