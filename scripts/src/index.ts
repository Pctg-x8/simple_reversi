declare function requestNextFrame(callback: () => void): void;
function nextFrame(): Promise<void> {
    return new Promise(resolve => requestNextFrame(resolve));
}

class Board {
    async run(): Promise<void> {
        while (true) {
            await nextFrame();
        }
    }
}

const board = new Board();
Promise.all([board.run()]);
