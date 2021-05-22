declare function requestNextFrame(callback: () => void): void;
function nextFrame(): Promise<void> {
    return new Promise(resolve => requestNextFrame(resolve));
}

class CellState {
    constructor(private stateFlags: number) {}

    get placed(): boolean {
        return (this.stateFlags & 0x80) != 0;
    }
    get white(): boolean {
        return (this.stateFlags & 0x01) != 0;
    }

    place(color: "white" | "black") {
        this.stateFlags = 0x80 | (color === "white" ? 0x01 : 0x00);
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

class Board {
    private state: BoardState = new BoardState();

    async run(): Promise<void> {
        this.state.dump();

        while (true) {
            await nextFrame();
        }
    }
}

const board = new Board();
Promise.all([board.run()]);
