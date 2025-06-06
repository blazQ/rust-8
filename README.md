# CHIP-8 Emulator

A simple CHIP-8 emulator written in Rust that runs in the terminal.

## Features

- Full CHIP-8 instruction set implementation
- Terminal-based display using Unicode blocks
- Configurable CPU frequency
- Real-time keyboard input
- Timer support (delay and sound timers)

## Usage

```bash
# Run with default settings (tetris.ch8 at 700Hz)
cargo run

# Load a specific ROM
cargo run pong.ch8
cargo run --rom space_invaders.ch8

# Adjust CPU speed
cargo run --tickcpu 1000

# Combine options
cargo run --rom breakout.ch8 --tickcpu 500

# Show help
cargo run --help
```

## Controls

The CHIP-8 keypad is mapped to your keyboard as follows:

```rust
CHIP-8 Keypad:    Your Keyboard:
1 2 3 C           1 2 3 4
4 5 6 D     →     Q W E R
7 8 9 E           A S D F
A 0 B F           Z X C V
```

Press **ESC** to exit the emulator.

## ROM Files

Place your CHIP-8 ROM files (`.ch8` files) in a `test_roms` directory or specify the full path:

```bash
cargo run test_roms/tetris.ch8
cargo run /path/to/your/game.ch8
```

## Requirements

- Rust (latest stable version)
- Terminal with Unicode support
- CHIP-8 ROM files

## Dependencies

- `crossterm` - For terminal input/output
- `rand` - For random number generation

## Building

```bash
git clone <your-repo>
cd chip8-emulator
cargo build --release
```

## Performance Notes

- Default CPU frequency: 700Hz
- Timer frequency: 60Hz (standard)
- Display refresh: ~60 FPS
- Adjust `--tickcpu` for different games (some may require faster/slower speeds)
- Still missing audio, will implement

## Compatibility

Implements the standard CHIP-8 instruction set. Should run most classic CHIP-8 games including:

- Tetris
- Pong
- Space Invaders
- Breakout
- And many others

---
