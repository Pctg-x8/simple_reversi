declare function requestNextFrame(callback: () => void): void;
function nextFrame(): Promise<void> {
    return new Promise((resolve) => requestNextFrame(resolve));
}
declare function isButtonPressing(): boolean;
declare function cursorPos(): [number, number];
declare function setBoardStateBuffer(buffer: ArrayBuffer): void;
declare function currentTimeMs(): number;

type StorableClassProperties<T> = {
    readonly SIZE: number;
    new (view: DataView): T;
};
class StorableStd140Array<T> {
    readonly buffer: ArrayBuffer;
    private readonly stride: number;

    constructor(
        private readonly cls: StorableClassProperties<T>,
        elements: number
    ) {
        // glsl std140 layout restriction
        this.stride = Math.trunc((cls.SIZE + 15) / 16) * 16;
        this.buffer = new ArrayBuffer(this.stride * elements);
    }

    get(index: number): T {
        return new this.cls(
            new DataView(this.buffer, this.stride * index, this.cls.SIZE)
        );
    }
}

class CellStateStorable {
    static readonly SIZE: number = 8;
    constructor(private readonly view: DataView) {}

    get stateFlags(): number {
        return this.view.getUint32(0, true);
    }
    set stateFlags(v: number) {
        this.view.setUint32(0, v, true);
    }
    get flipStartTime(): number {
        return this.view.getFloat32(4, true);
    }
    set flipStartTime(v: number) {
        this.view.setFloat32(4, v, true);
    }
}
class CellState extends CellStateStorable {
    init() {
        this.stateFlags = 0;
        this.flipStartTime = 0;
    }

    get placed(): boolean {
        return (this.stateFlags & 0x80) != 0;
    }
    get white(): boolean {
        return (this.stateFlags & 0x01) != 0;
    }
    get color(): "white" | "black" {
        return this.white ? "white" : "black";
    }

    place(color: "white" | "black") {
        this.stateFlags = 0x80 | (color === "white" ? 0x01 : 0x00);
    }

    flip() {
        if (!this.placed) return;
        this.stateFlags ^= 0x01;
    }

    beginFlipAnimation(at: number) {
        this.flipStartTime = at;
    }
}
const AROUND_DIRECTIONS = [
    [-1, -1],
    [0, -1],
    [1, -1],
    [-1, 0],
    [1, 0],
    [-1, 1],
    [0, 1],
    [1, 1],
];
class BoardState {
    // for std140 uniform layout
    private cells = new StorableStd140Array(CellState, 8 * 8);
    private whiteCounter = 2;
    private blackCounter = 2;

    constructor() {
        for (let y = 0; y < 8; y++) {
            for (let x = 0; x < 8; x++) {
                this.cells.get(x + y * 8).init();
            }
        }
        this.cell(3, 3)!.place("black");
        this.cell(4, 4)!.place("black");
        this.cell(4, 3)!.place("white");
        this.cell(3, 4)!.place("white");
    }

    cell(x: number, y: number): CellState | undefined {
        if (0 <= x && x < 8 && 0 <= y && y < 8) {
            return this.cells.get(x + y * 8);
        }
    }

    /** Returns true if successfully placed the stone, escapes frame for motion */
    async place(
        x: number,
        y: number,
        color: "white" | "black"
    ): Promise<boolean> {
        const c = this.cell(x, y);
        if (!c) return false;
        if (c.placed) return false;
        c.place(color);
        if (color === "white") {
            this.whiteCounter++;
        } else {
            this.blackCounter++;
        }
        const flipDirections: [number, number, number][] =
            AROUND_DIRECTIONS.flatMap(([dx, dy]) => {
                const flipCount = this.findFlipCount(x, y, dx, dy, color);
                if (!flipCount) return [];
                return [[dx, dy, flipCount]];
            });
        const flipDirectionMax = flipDirections.reduce(
            (a, [, , c]) => Math.max(a, c),
            0
        );
        for (let mag = 1; mag <= flipDirectionMax; mag++) {
            for (const [dx, dy, max] of flipDirections) {
                if (max < mag) continue;
                const c = this.cell(x + dx * mag, y + dy * mag)!;
                c.flip();
                c.beginFlipAnimation(currentTimeMs());
                if (color === "white") {
                    this.whiteCounter++;
                    this.blackCounter--;
                } else {
                    this.blackCounter++;
                    this.whiteCounter--;
                }
            }
            this.syncStateBuffer();
            for (let i = 0; i < 4; i++) await nextFrame();
        }
        this.syncStateBuffer();
        return true;
    }

    syncStateBuffer() {
        setBoardStateBuffer(this.cells.buffer);
    }

    get hasGameFinished(): boolean {
        return this.blackCounter + this.whiteCounter >= 64;
    }

    /** null = draw */
    get winSide(): "white" | "black" | null {
        if (this.blackCounter == this.whiteCounter) return null;
        return this.blackCounter > this.whiteCounter ? "black" : "white";
    }
    get scoreboardText(): string {
        return `white ${this.whiteCounter} black ${this.blackCounter}`;
    }

    findLegalPlacePositions(color: "white" | "black"): [number, number][] {
        let positions: [number, number][] = [];
        for (let y = 0; y < 8; y++) {
            for (let x = 0; x < 8; x++) {
                if (this.cell(x, y)!.placed) continue;
                const flipCounts = AROUND_DIRECTIONS.map(([dx, dy]) =>
                    this.findFlipCount(x, y, dx, dy, color)
                ).filter(Boolean);
                if (flipCounts.length > 0) positions.push([x, y]);
            }
        }

        return positions;
    }

    private findFlipCount(
        x: number,
        y: number,
        dx: number,
        dy: number,
        color: "white" | "black"
    ): number | undefined {
        const c2 = this.cell(x + dx, y + dy);
        if (!c2 || !c2.placed || c2.color === color) return;
        let mag = 2;
        while (true) {
            const c3 = this.cell(x + dx * mag, y + dy * mag);
            if (!c3 || !c3.placed) return;
            if (c3.color === color) return mag - 1;
            mag += 1;
        }
    }

    dump() {
        let str = this.scoreboardText;
        for (let y = 0; y < 8; y++) {
            str += "\n";
            for (let x = 0; x < 8; x++) {
                const c = this.cell(x, y)!;
                if (!c.placed) {
                    str += "_";
                } else {
                    str += c.white ? "W" : "B";
                }
            }
        }
        console.log(str);
    }
}

class EdgeTrigger<T> {
    constructor(private value: T) {}

    get current(): T {
        return this.value;
    }

    update(newvalue: T): boolean {
        const changed = this.value !== newvalue;
        this.value = newvalue;
        return changed;
    }
}

class BoardControl {
    private state = new BoardState();
    private buttonPressEdge = new EdgeTrigger(false);
    private currentPhase: "black" | "white" = "white";
    private legalPlacePositions: [number, number][] = [];

    async run(): Promise<void> {
        const aroundMargin = 480 * (1.0 - 0.78) * 0.5;
        const boardSize = 480 - aroundMargin * 2;
        const cellSize = boardSize / 8;
        console.log(`aroundMargin: ${aroundMargin}`);
        // to initialize internal states
        this.state.syncStateBuffer();
        this.flipTurn();

        while (!this.state.hasGameFinished) {
            if (
                this.buttonPressEdge.update(isButtonPressing()) &&
                this.buttonPressEdge.current
            ) {
                const [cx, cy] = cursorPos();
                const [bx, by] = [cx - aroundMargin, cy - aroundMargin];
                if (0 <= bx && bx < boardSize && 0 <= by && by < boardSize) {
                    const [cellX, cellY] = [
                        Math.trunc(bx / cellSize),
                        Math.trunc(by / cellSize),
                    ];
                    if (this.isLegalPlacePosition(cellX, cellY)) {
                        console.log(`place time ${currentTimeMs()}`);
                        await this.state.place(cellX, cellY, this.currentPhase);
                        if (!this.state.hasGameFinished) {
                            do {
                                this.flipTurn();
                            } while (this.legalPlacePositions.length <= 0);
                        }
                    }
                }
            }

            await nextFrame();
        }
        console.log("Game finished");
        const ws = this.state.winSide;
        console.log(
            `${ws === null ? "draw" : ws + " win"} (${
                this.state.scoreboardText
            })`
        );
    }

    private isLegalPlacePosition(x: number, y: number): boolean {
        return (
            this.legalPlacePositions.find(([px, py]) => px == x && py == y) !==
            undefined
        );
    }

    private flipTurn() {
        this.currentPhase = this.currentPhase === "white" ? "black" : "white";
        this.state.dump();
        console.log(`${this.currentPhase} phase`);
        this.legalPlacePositions = this.state.findLegalPlacePositions(
            this.currentPhase
        );
    }
}

const board = new BoardControl();
Promise.all([board.run()]);
