declare function requestNextFrame(callback: () => void): void;
function nextFrame(): Promise<void> {
    return new Promise(resolve => requestNextFrame(resolve));
}
declare function isButtonPressing(): boolean;
declare function cursorPos(): [number, number];

class CellState {
    constructor(private stateFlags: number) {}

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
}
class BoardState {
    private cells: CellState[] = [];

    constructor() {
        for (let y = 0; y < 8; y++) {
            for (let x = 0; x < 8; x++) {
                this.cells[x + y * 8] = new CellState(0);
            }
        }
        this.cell(3, 3)!.place("black");
        this.cell(4, 4)!.place("black");
        this.cell(4, 3)!.place("white");
        this.cell(3, 4)!.place("white");
    }

    cell(x: number, y: number): CellState | undefined {
        if (0 <= x && x < 8 && 0 <= y && y < 8) return this.cells[x + y * 8];
    }

    place(x: number, y: number, color: "white" | "black"): boolean {
        const c = this.cell(x, y);
        if (!c) return false;
        if (c.placed) return false;
        c.place(color);
        const DIRECTIONS = [[-1, -1], [0, -1], [1, -1], [-1, 0], [1, 0], [-1, 1], [0, 1], [1, 1]];
        DIRECTIONS.forEach(([dx, dy]) => {
            const flipCount = this.findFlipCount(x, y, dx, dy, color);
            if (!flipCount) return;
            for (let mag = 1; mag <= flipCount; mag++) {
                this.cell(x + dx * mag, y + dy * mag)!.flip();
            }
        })
        return true;
    }

    findFlipCount(x: number, y: number, dx: number, dy: number, color: "white" | "black"): number | undefined {
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
        let str = "";
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

    get current(): T { return this.value; }

    update(newvalue: T): boolean {
        const changed = this.value !== newvalue;
        this.value = newvalue;
        return changed;
    }
}

class Board {
    private state = new BoardState();
    private buttonPressEdge = new EdgeTrigger(false);
    private currentPhase: "black" | "white" = "black";

    async run(): Promise<void> {
        const aroundMargin = 480 * (1.0 - 0.78) * 0.5;
        const boardSize = 480 - aroundMargin * 2;
        const cellSize = boardSize / 8;
        console.log(`aroundMargin: ${aroundMargin}`);
        this.state.dump();
        console.log(`${this.currentPhase} phase`);

        while (true) {
            if (this.buttonPressEdge.update(isButtonPressing()) && this.buttonPressEdge.current) {
                const [cx, cy] = cursorPos();
                const [bx, by] = [cx - aroundMargin, cy - aroundMargin];
                if (0 <= bx && bx < boardSize && 0 <= by && by < boardSize) {
                    const [cellX, cellY] = [Math.trunc(bx / cellSize), Math.trunc(by / cellSize)];
                    if (this.state.place(cellX, cellY, this.currentPhase)) {
                        this.currentPhase = this.currentPhase === "white" ? "black" : "white";
                        this.state.dump();
                        console.log(`${this.currentPhase} phase`);
                    }
                }
            }

            await nextFrame();
        }
    }
}

const board = new Board();
Promise.all([board.run()]);
