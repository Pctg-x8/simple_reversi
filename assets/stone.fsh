#version 150

in float yref;

void main() {
    float bw = step(yref, 0.5);
    gl_FragColor = vec4(bw, bw, bw, 1.0);
}
