# Vanishing Tic Tac Toe

Local pass-and-play tic tac toe in Rust + Macroquad, with a twist: each player may have at most 3 marks on the board.

**Play it in the browser:** https://sriman97.github.io/vanishing-tic-tac-toe/ (works on phones; add it to the Home Screen from Safari's Share menu).

## Rules
- 3x3 board, X starts, one device.
- Placing a 4th mark removes your oldest mark first (FIFO). Then the clicked cell must be empty (your own oldest cell counts as freed).
- Three in a row wins (checked after the removal). Draws are impossible.
- When you have 3 marks, your oldest is drawn faded so you can see what will vanish.
- Tap/click after a win to play again.

## Desktop
    cargo run --release
Works on macOS, Windows and Linux.

## Tests
    cargo test

## Web (WASM)
    rustup target add wasm32-unknown-unknown
    cargo build --release --target wasm32-unknown-unknown
    cp target/wasm32-unknown-unknown/release/vanishing-ttt.wasm web/vanishing_ttt.wasm
    cd web && python3 -m http.server   # open http://localhost:8000

`web/mq_js_bundle.js` is the official Macroquad loader. For a single-file build run
`python3 make_standalone.py` (after copying the wasm) to produce `web/standalone.html`.
GitHub Pages serves `docs/index.html`, which is a copy of that file; copy it over after each web build.

## Font
Text uses Inter SemiBold (`assets/Inter-SemiBold.otf`, subset to ASCII), embedded into the binary at build time. Inter is licensed under the SIL Open Font License 1.1; see `assets/OFL.txt`.
