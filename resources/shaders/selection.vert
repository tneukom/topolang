#version 130

in vec2 in_glwindow_position;

out vec2 frag_texcoord;

void main() {
    gl_Position = vec4(in_glwindow_position, 0.0, 1.0);
}