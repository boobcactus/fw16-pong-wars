# FW16 Pong Wars

A Rust app that plays Pong Wars on the Framework Laptop 16 LED Matrix over USB serial.

## Requirements

- At least one Framework LED Matrix Input Module
- Optional second module for dual-mode (-d, --dualmode)
- Rust toolchain (stable cargo + rustc)

## Run

```bash
cargo run --release -- [FLAGS]
```

Flags

- `-d`, `--dualmode`  Drive two modules side-by-side (18x34)
- `-s`, `--speed <1-64>`  Target FPS (default 32)
    - Supports up to 124 FPS with current baud rate, but may lead to instability in the EC.
- `-b`, `--brightness <0-100>`  Brightness percent (default 50)
- `--debug`  Extra timing/log output

Example

```bash
cargo run --release -- --dualmode --speed 48 --brightness 70
```

Controls

- `Ctrl+C` to exit

Gameplay

- Two balls (Day and Night) bounce and flip tiles to their color
- Tiles are lit for Day and dark for Night; balls render as the inverse of their color for visibility

## License

MIT License

## Acknowledgments

- Original Pong Wars by Koen van Gilst: https://github.com/vnglst/pong-wars
- Framework Computer for the LED Matrix hardware and open-source firmware
