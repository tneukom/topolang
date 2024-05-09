#version 300 es

uniform sampler2D tile_atlas_texture;
uniform highp float time;

in highp vec2 frag_texcoord;

out highp vec4 out_color;

void main() {
    highp vec2 window_frag_coord = gl_FragCoord.xy;
    highp vec4 texcolor = texture(tile_atlas_texture, frag_texcoord);
    // Wave effect
    const highp float PI = 3.141592653589;
    const highp vec3 RULE_FRAME = vec3(54.0 / 255.0, 12.0 / 255.0, 41.0 / 255.0);
    const highp vec3 RULE_ARROW = vec3(255.0 / 255.0, 110.0 / 255.0, 0.0 / 255.0);

    if(texcolor.rgb == RULE_FRAME || texcolor.rgb == RULE_ARROW) {
        highp float s = 1.0/10.0 * PI * (window_frag_coord.x + window_frag_coord.y);
        highp float c = sin(s + 5.0 * time) > 0.0 ? 1.0 : 0.7;
        out_color = vec4(c * texcolor.a * texcolor.rgb, texcolor.a);
    } else {
        out_color = vec4(texcolor.a * texcolor.rgb, texcolor.a);
    }

}