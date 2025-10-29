   # FW16 Pong Wars

A Rust app that plays Pong Wars on the Framework Laptop 16 LED Matrix.

https://github.com/user-attachments/assets/2d7a4b85-f580-4dbc-9378-3473213b643f

## Requirements

- At least one [Framework Laptop 16 LED Matrix](https://frame.work/products/16-led-matrix)
- Optional second LED Matrix for dual-mode (-d, --dualmode)
- Rust toolchain (stable cargo + rustc)

## Run

```bash
cargo run --release -- [FLAGS]
```

Flags

- `-d`, `--dualmode`  Drive two modules side-by-side (18x34)
- `-b`, `--balls [1-5]`  Balls per team. Defaults to 2 if no number is provided.
- `-s`, `--speed <1-64>`  Target FPS (default 64)
    - Supports up to 124 FPS by editing [this value](https://github.com/boobcactus/fw16-pong-wars/blob/b246b33519e5e006077fbc7d48cc27122e02981f/src/main.rs#L21), but may lead to instability in the EC.
- `-B`, `--brightness <0-100>`  Brightness percent (default 50)
- `--debug`  Extra timing/log output

Example

```bash
cargo run --release -- --dualmode --speed 48 --brightness 70 --balls 4
```

## License

MIT License

## Acknowledgments

- Original Pong Wars by Koen van Gilst: https://github.com/vnglst/pong-wars
- Framework Computer for the LED Matrix hardware and open-source firmware
- Windsurf and OpenAI's GPT-5 enabling me to bring this idea to life  
