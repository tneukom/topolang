#version 300 es

// The vertex position is in pixelwindow coordinates
in highp vec2 glwindow_position;

out highp vec2 frag_position;

uniform highp mat3 glwindow_to_pixelwindow;

void main() {
    frag_position = (glwindow_to_pixelwindow * vec3(glwindow_position, 1.0)).xy;
    gl_Position = vec4(glwindow_position, 0.0, 1.0);
}