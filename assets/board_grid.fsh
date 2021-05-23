#version 150

in vec2 uv;
out vec4 frag_color;

void main() {
    vec2 rep = fract(uv * 8);
    float edge_low = min(smoothstep(rep.x, 0.0, 0.01), smoothstep(rep.y, 0.0, 0.01));
    float edge_high = min(smoothstep(rep.x, 0.99, 1.0), smoothstep(rep.y, 0.99, 1.0));
    frag_color = vec4(0.0, 0.65 * min(edge_low, edge_high), 0.0, 1.0);
}
