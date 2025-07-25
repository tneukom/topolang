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

bool is_alpha_boundary(float center_a) {
    // Don't apply translation
    vec2 uv_x = 2.0 * (view_to_gltexture * vec3(1.0, 0.0, 0.0)).xy;
    vec2 uv_y = 2.0 * (view_to_gltexture * vec3(0.0, 1.0, 0.0)).xy;

    float top_a = texture(material_texture, gltexture_uv - uv_y).a;
    float left_a = texture(material_texture, gltexture_uv - uv_x).a;
    float bottom_a = texture(material_texture, gltexture_uv + uv_y).a;
    float right_a = texture(material_texture, gltexture_uv + uv_x).a;
    float top_left_a = texture(material_texture, gltexture_uv - uv_y - uv_x).a;
    float top_right_a = texture(material_texture, gltexture_uv - uv_y + uv_x).a;
    float bottom_left_a = texture(material_texture, gltexture_uv + uv_y - uv_x).a;
    float bottom_right_a = texture(material_texture, gltexture_uv + uv_y + uv_x).a;

    return distance(center_a, top_a) > 0.0
        || distance(center_a, left_a) > 0.0
        || distance(center_a, bottom_a) > 0.0
        || distance(center_a, right_a) > 0.0
        || distance(center_a, top_left_a) > 0.0
        || distance(center_a, top_right_a) > 0.0
        || distance(center_a, bottom_left_a) > 0.0
        || distance(center_a, bottom_right_a) > 0.0;
}

void main() {
    // material_texture is has format SRGB8_ALPHA8, so RGB is linearized
    vec4 color = texture(material_texture, gltexture_uv);


    if(color.a > 253.5/255.0 && color.a < 254.5/255.0) {
        // Solid alpha 254
        if(is_alpha_boundary(color.a)) {
            vec4 even_color = vec4(1.0, 0.8, 0.0, 1.0);
            vec4 odd_color = vec4(0.0, 0.0, 0.0, 1.0);
            bool even = mod(gl_FragCoord.x + gl_FragCoord.y + 6.0 * time, 16.0) < 8.0;
            out_color = even ? even_color : odd_color;
        } else {
            out_color = vec4(srgb_from_linear(color.rgb), 1.0);
        }
    } else if(color.a > 130.5/255.0 && color.a < 131.5/255.0) {
        // Sleeping alpha 131
        if(is_alpha_boundary(color.a)) {
            vec4 even_color = vec4(0.0, 1.0, 1.0, 1.0);
            vec4 odd_color = vec4(1.0, 1.0, 1.0, 1.0);
            bool even = mod(gl_FragCoord.x + gl_FragCoord.y + 6.0 * time, 16.0) < 8.0;
            out_color = even ? even_color : odd_color;
        } else {
            out_color = vec4(srgb_from_linear(color.rgb), 200.0);
        }
    } else {
        out_color = vec4(srgb_from_linear(color.rgb), color.a);
    }
}