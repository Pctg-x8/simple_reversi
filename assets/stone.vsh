#version 150

in vec4 pos;
out float yref;
uniform mat4 world_transform;
uniform float time_ms;

const float SPACE = 2.55;

struct CellState {
    uint stateFlags;
    float flipStartTime;
};
layout(std140) uniform BoardState {
    CellState cells[8 * 8];
} boardState;
bool cellPlaced(CellState c) {
    return (int(c.stateFlags) & 0x80) != 0;
}
bool cellIsWhite(CellState c) {
    return (int(c.stateFlags) & 0x01) != 0;
}

void main() {
    CellState cell = boardState.cells[gl_InstanceID];
    float a = cellIsWhite(cell) ? 0.0 : 3.1415926;
    mat4 rot = mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, cos(a),-sin(a), 0.0,
        0.0, sin(a), cos(a), 0.0,
        0.0, 0.0, 0.0, 1.0
    );
    vec4 o = vec4((gl_InstanceID % 8 - 4 + 0.5) * SPACE, -(gl_InstanceID / 8 - 4 + 0.5) * SPACE, sin(time_ms * 0.01) * 12.0, 0.0);
    vec4 s = cellPlaced(cell) ? vec4(1.0, 1.0, 0.01, 1.0) : vec4(0.0);
    gl_Position = (pos * rot + o) * s * world_transform;
    yref = pos.z;
}
