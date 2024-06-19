This is a minimal implementation of the CHIP-8 interpreter in Rust.

📢: If you run this program in a linux server like me, you can't get a full experience, 
because in the linux server, the terminal is not support to capture the key release event.
Eventually, I tested this program in windows desktop and linux desktop, and it works well.

## Usage

```bash
cargo run --package shinobu8-tui -- --rom ./roms/15PUZZLE
```

Refer:

- https://github.com/aquova/chip8-book/
- http://devernay.free.fr/hacks/chip8/C8TECH10.HTM