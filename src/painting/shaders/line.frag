#version 300 es
precision highp float;

uniform float time;

out vec4 out_color;

void main() {
    // Animated diagonally stripped lines
    float alpha = mod(gl_FragCoord.x + gl_FragCoord.y + 10.0 * time, 10.0) < 5.0 ? 1.0 : 0.0;
    out_color = vec4(0.0, 0.0, 0.0, alpha);
}