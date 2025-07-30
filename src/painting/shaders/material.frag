#version 300 es
precision highp float;

uniform sampler2D material_texture;
uniform float time;
uniform mat3 world_to_view;

in vec2 pass_gltexture_uv;
in vec2 pass_world_position;

out vec4 out_color;

// https://gamedev.stackexchange.com/questions/92015/optimized-linear-to-srgb-glsl
vec3 srgb_from_linear(vec3 linear_rgb)
{
    bvec3 cutoff = lessThan(linear_rgb, vec3(0.0031308));
    vec3 higher = vec3(1.055) * pow(linear_rgb, vec3(1.0 / 2.4)) - vec3(0.055);
    vec3 lower = linear_rgb * vec3(12.92);

    return mix(higher, lower, cutoff);
}

// https://gist.github.com/983/e170a24ae8eba2cd174f
vec3 rgb_to_hsv(vec3 c)
{
    vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));
    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

vec3 hsv_to_rgb(vec3 c)
{
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

// vec3 rgb_to_yuv(vec3 rgb) {
//     // mat3 is column major!
//     mat3 M = transpose(mat3(
//         0.299,    0.587,    0.114,
//        -0.14713, -0.28886,  0.436,
//         0.615,   -0.51499, -0.10001
//     ));
//     return M * rgb;
// }

// vec3 yuv_to_rgb(vec3 yuv) {
//     mat3 N = transpose(mat3(
//         1.0,  0.0,      1.13983,
//         1.0, -0.39465, -0.58060,
//         1.0,  2.03211,  0.0
//     ));
//     return N * yuv;
// }


void main() {
    // material_texture is has format SRGB8_ALPHA8, so RGB is linearized
    vec4 color = texture(material_texture, pass_gltexture_uv);

    // Assuming uniform scale
    float scale_world_to_view = world_to_view[0][0];

    vec3 srgb_color = srgb_from_linear(color.rgb);
    if(253.5/255.0 < color.a && color.a < 254.5/255.0) {
        // solid
        float t = scale_world_to_view * (pass_world_position.x + pass_world_position.y);
        if(mod(t, 10.0 * sqrt(2.0)) < 5.0 * sqrt(2.0)) {
            // if done in linear space, the increase by 0.1 appears much stronger close to 0 than close to 1
            // Changing the v component in HSV color space looks better over all colors than changing Y in YUV
            // color space.
            vec3 hsv = rgb_to_hsv(srgb_color.rgb);
            hsv.z = hsv.z < 0.5 ? hsv.z + 0.15 : hsv.z - 0.15;
            srgb_color.rgb = hsv_to_rgb(hsv);
            // vec3 yuv = rgb_to_yuv(srgb_color.rgb);
            // yuv.x = yuv.x < 0.5 ? yuv.x + 0.1 : yuv.x - 0.1;
            // srgb_color.rgb = yuv_to_rgb(yuv);
        }
    } else if(130.5/255.0 < color.a && color.a < 131.5/255.0) {
        // sleeping
        float x = scale_world_to_view * pass_world_position.x;
        float t = scale_world_to_view * pass_world_position.y - sin(x / 2.0);
        if(mod(t, 10.0) < 5.0) {
            color.a = 130.0/255.0;
        } else {
            color.a = 190.0/255.0;
        }
    }

    out_color = vec4(srgb_color, color.a);
}