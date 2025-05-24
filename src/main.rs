use anyhow::Result;
use std::thread;
use std::time::{Duration, Instant};
use std::env;

mod game;
mod led_matrix;

use game::GameState;
use led_matrix::LedMatrix;

fn main() -> Result<()> {
    println!("Starting FW16 Pong Wars...");
    
    // Check command line arguments for mode
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("help");
    
    match mode {
        "live" => {
            println!("Running in live mode...");
            run_live_mode()
        }
        "simulation" | "sim" => {
            println!("Running in simulation mode...");
            run_simulation()
        }
        "check" => {
            println!("Scanning for Framework LED Matrix modules...\n");
            match LedMatrix::find_all_devices() {
                Ok(devices) => {
                    if devices.is_empty() {
                        println!("✗ No Framework LED Matrix modules found.");
                        println!("\nMake sure:");
                        println!("  - The LED Matrix module is properly connected");
                        println!("  - The module is powered on");
                        println!("  - You have the necessary permissions to access USB devices");
                    } else {
                        println!("✓ Found {} LED Matrix module(s):\n", devices.len());
                        for (idx, (port_info, device_info)) in devices.iter().enumerate() {
                            println!("Module #{}:", idx + 1);
                            println!("  Port: {}", port_info.port_name);
                            println!("  Info: {}", device_info);
                            println!();
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    println!("✗ Error scanning for devices: {}", e);
                    Ok(())
                }
            }
        }
        "help" | "-h" | "--help" | _ => {
            print_help();
            Ok(())
        }
    }
}

fn print_help() {
    println!("FW16 Pong Wars - A Pong Wars game for Framework 16 LED Matrix");
    println!();
    println!("USAGE:");
    println!("    fw16-pong-wars [COMMAND]");
    println!();
    println!("COMMANDS:");
    println!("    live, -l         Run in live mode (requires LED Matrix)");
    println!("    simulation, sim  Run in simulation mode (console output)");
    println!("    check            Check if LED Matrix is available");
    println!("    help, -h         Display this help message");
    println!();
    println!("EXAMPLES:");
    println!("    fw16-pong-wars           # Show help");
    println!("    fw16-pong-wars sim       # Run simulation");
    println!("    fw16-pong-wars live      # Run on LED Matrix");
    println!("    fw16-pong-wars check     # Check LED Matrix availability");
    println!();
    println!("NOTES:");
    println!("    - Live mode requires the Framework 16 LED Matrix module");
    println!("    - Simulation mode runs in the console for testing");
    println!("    - Press Ctrl+C to exit the game");
}

fn run_live_mode() -> Result<()> {
    // Check if LED Matrix is available
    println!("Checking for LED Matrix...");
    LedMatrix::check_available()?;
    
    // Connect to LED Matrix
    println!("Connecting to LED Matrix...");
    let mut led_matrix = LedMatrix::new()?;
    println!("LED Matrix connected!");
    
    // Initialize the display
    led_matrix.init_display()?;
    println!("LED Matrix initialized!");
    
    // Initialize game state
    let mut game_state = GameState::new();
    
    // Game loop timing
    let frame_duration = Duration::from_millis(100); // 10 FPS
    let mut last_frame = Instant::now();
    
    println!("Game started! Press Ctrl+C to exit.");
    
    loop {
        // Check if enough time has passed for the next frame
        let now = Instant::now();
        if now.duration_since(last_frame) >= frame_duration {
            last_frame = now;
            
            // Update game state
            game_state.update();
            
            // Render to LED matrix
            if let Err(e) = led_matrix.render(&game_state) {
                eprintln!("Error rendering to LED matrix: {}", e);
                // Continue running even if render fails
            }
            
            // Print scores (less verbose in live mode)
            if game_state.day_score % 10 == 0 || game_state.night_score % 10 == 0 {
                println!("Day: {} | Night: {}", game_state.day_score, game_state.night_score);
            }
        }
        
        // Small sleep to prevent busy waiting
        thread::sleep(Duration::from_millis(10));
    }
}

fn run_simulation() -> Result<()> {
    // Initialize game state
    let mut game_state = GameState::new();
    
    // Game loop timing
    let frame_duration = Duration::from_millis(100); // 10 FPS
    let mut last_frame = Instant::now();
    
    println!("Running in simulation mode. Press Ctrl+C to exit.");
    
    loop {
        // Check if enough time has passed for the next frame
        let now = Instant::now();
        if now.duration_since(last_frame) >= frame_duration {
            last_frame = now;
            
            // Update game state
            game_state.update();
            
            // Print game state to console
            print_game_state(&game_state);
        }
        
        // Small sleep to prevent busy waiting
        thread::sleep(Duration::from_millis(10));
    }
}

fn print_game_state(game_state: &GameState) {
    // Clear screen (Windows)
    print!("\x1B[2J\x1B[1;1H");
    
    println!("FW16 Pong Wars - Simulation Mode");
    println!("Day: {} | Night: {}", game_state.day_score, game_state.night_score);
    println!();
    
    // Print the grid
    for y in 0..game::GRID_HEIGHT {
        for x in 0..game::GRID_WIDTH {
            // Check if ball is here
            let mut has_ball = false;
            for ball in &game_state.balls {
                if ball.x as usize == x && ball.y as usize == y {
                    has_ball = true;
                    break;
                }
            }
            
            if has_ball {
                print!("●");
            } else {
                match game_state.squares[x][y] {
                    game::SquareColor::Day => print!("□"),
                    game::SquareColor::Night => print!("■"),
                }
            }
        }
        println!();
    }
}
