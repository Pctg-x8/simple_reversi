#version 150

in float yref;
out vec4 frag_color;

void main() {
    float bw = step(yref, 0.5);
    frag_color = vec4(bw, bw, bw, 1.0);
}
