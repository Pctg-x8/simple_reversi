declare function requestNextFrame(callback: () => void): void;

console.log("hello from script");
requestNextFrame(() => console.log("resume!"));
