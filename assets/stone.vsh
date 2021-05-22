#version 150

in vec4 pos;
out float yref;
uniform mat4 world_transform;

void main() {
    gl_Position = pos * world_transform;
    yref = pos.z;
}
