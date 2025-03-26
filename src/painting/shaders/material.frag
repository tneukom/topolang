#version 300 es
precision highp float;

uniform sampler2D material_texture;

in vec2 gltexture_uv;

out vec4 out_color;

// https://gamedev.stackexchange.com/questions/92015/optimized-linear-to-srgb-glsl
vec4 srgb_from_linear(vec4 linear_rgba)
{
    bvec3 cutoff = lessThan(linear_rgba.rgb, vec3(0.0031308));
    vec3 higher = vec3(1.055) * pow(linear_rgba.rgb, vec3(1.0 / 2.4)) - vec3(0.055);
    vec3 lower = linear_rgba.rgb * vec3(12.92);

    return vec4(mix(higher, lower, cutoff), linear_rgba.a);
}

void main() {
    vec4 color = texture(material_texture, gltexture_uv);
    out_color = srgb_from_linear(color);
}