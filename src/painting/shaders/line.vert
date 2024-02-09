#version 300 es

in highp vec2 in_glwindow_position;

out highp vec2 frag_texcoord;

void main() {
    gl_Position = vec4(in_glwindow_position, 0.0, 1.0);
}