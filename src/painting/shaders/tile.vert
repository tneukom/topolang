#version 300 es

in highp vec2 in_glwindow_position;
in highp vec2 in_texcoord;

out highp vec2 frag_texcoord;

void main() {
    frag_texcoord = in_texcoord;
    gl_Position = vec4(in_glwindow_position, 0.0, 1.0);
}