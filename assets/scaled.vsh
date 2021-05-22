#version 150

in vec2 pos;
out vec2 uv;
uniform float scale;

void main() {
    gl_Position = vec4(pos * scale, 0.0, 1.0);
    uv = pos * 0.5 + 0.5;
}
