#version 300 es
precision highp float;

uniform sampler2D material_texture;
uniform float time;
uniform mat3 view_to_gltexture;

in vec2 gltexture_uv;

out vec4 out_color;

// https://gamedev.stackexchange.com/questions/92015/optimized-linear-to-srgb-glsl
vec3 srgb_from_linear(vec3 linear_rgb)
{
    bvec3 cutoff = lessThan(linear_rgb, vec3(0.0031308));
    vec3 higher = vec3(1.055) * pow(linear_rgb, vec3(1.0 / 2.4)) - vec3(0.055);
    vec3 lower = linear_rgb * vec3(12.92);

    return mix(higher, lower, cutoff);
}

void main() {
    // material_texture is has format SRGB8_ALPHA8, so RGB is linearized
    vec4 color = texture(material_texture, gltexture_uv);

    out_color = vec4(srgb_from_linear(color.rgb), color.a);
}