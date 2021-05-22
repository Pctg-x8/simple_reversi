#version 150

in vec4 pos;
out float yref;
uniform mat4 world_transform;

const float SPACE = 2.55;

void main() {
    vec4 o = vec4((gl_InstanceID % 8 - 4 + 0.5) * SPACE, -(gl_InstanceID / 8 - 4 + 0.5) * SPACE, 0.0, 0.0);
    vec4 s = vec4(1.0, 1.0, 0.01, 1.0);
    gl_Position = (pos + o) * s * world_transform;
    yref = pos.z;
}
