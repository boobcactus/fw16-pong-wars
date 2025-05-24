# FW16 Pong Wars

A recreation of Pong Wars for the Framework 16 LED Matrix module. Watch as two teams (Day and Night) battle it out in an endless game of territorial pong!

## Features

- **Simulation Mode**: Terminal-based visualization of the game
- **Live Mode**: Display the game on the physical Framework 16 LED Matrix module
- Two balls bouncing around claiming territory for their teams
- Dynamic scoring based on claimed squares
- System tray support for live mode

## Requirements

- Framework 16 laptop with LED Matrix module (for live mode)
- Rust toolchain

## Usage

### Simulation Mode (Default)
```bash
cargo run
```
or explicitly:
```bash
cargo run -- simulation
```

This runs the game in your terminal with ASCII visualization. No LED hardware required.

### Live Mode
```bash
cargo run -- live
```

This mode:
- Connects to the Framework 16 LED Matrix module
- Displays the game on the physical LEDs
- Creates a system tray icon
- Runs in the background (use system tray to exit)

## Game Rules

- Two balls (Day and Night) bounce around the 9x34 grid
- When a ball touches a square, it claims it for their team
- Day squares appear bright, Night squares appear dim
- Balls bounce off edges and continue endlessly
- Score is the count of squares each team controls

## Building

```bash
cargo build --release
```

## Technical Details

- Uses column-based grayscale rendering for the LED Matrix
- 10 FPS update rate
- USB Serial communication at 115200 baud
- Individual pixel brightness control (0-255)

## Future Enhancements

- [ ] Configuration options
- [ ] Multiple game modes
- [ ] RGB LED Matrix support
- [ ] Linux/macOS support

## License

MIT License

## Acknowledgments

- Original Pong Wars by [Koen van Gilst](https://github.com/vnglst/pong-wars)
- Framework Computer for the LED Matrix module and documentation
