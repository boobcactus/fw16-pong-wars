mod game;
mod led_matrix;
mod system_tray;

use anyhow::Result;
use std::thread;
use std::time::{Duration, Instant};
use std::io::{self, Write};
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossterm::{
    cursor,
    execute,
    terminal::{self, ClearType},
    style::{Color, Print, SetForegroundColor, ResetColor},
};

use game::GameState;
use led_matrix::LedMatrix;
use system_tray::{SystemTray, TrayCommand};

const DEFAULT_FPS: f64 = 10.0;
const MIN_FPS: f64 = 1.0;
const MAX_FPS: f64 = 60.0;
const DISPLAY_FPS: f64 = 30.0;
const LED_SLEEP_MS: u64 = 1;

fn main() -> Result<()> {
    println!("Starting FW16 Pong Wars...");
    
    let args: Vec<String> = env::args().collect();
    
    let mut mode = "help";
    let mut fps = DEFAULT_FPS;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--fps" | "-f" => {
                if i + 1 < args.len() {
                    if args.get(i - 1).map(|s| s.as_str()) == Some("sim") || 
                       args.get(i - 1).map(|s| s.as_str()) == Some("simulation") {
                        fps = args[i + 1].parse::<f64>().unwrap_or(DEFAULT_FPS).clamp(MIN_FPS, MAX_FPS);
                        i += 2;
                    } else {
                        eprintln!("Error: --fps is only supported in simulation mode");
                        return Ok(());
                    }
                } else {
                    eprintln!("Error: --fps requires a value");
                    return Ok(());
                }
            }
            arg => {
                mode = arg;
                i += 1;
            }
        }
    }
    
    match mode {
        "live" | "-l" => {
            println!("Running in live mode...");
            run_live_mode()
        }
        "simulation" | "sim" => {
            println!("Running in simulation mode at {} FPS...", fps);
            run_simulation_mode(fps)
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
    println!("    fw16-pong-wars [OPTIONS] [COMMAND]");
    println!();
    println!("COMMANDS:");
    println!("    live, -l         Run in live mode (requires LED Matrix)");
    println!("    simulation, sim  Run in simulation mode (console output)");
    println!("    check            Check if LED Matrix is available");
    println!("    help, -h         Display this help message");
    println!();
    println!("SIMULATION MODE OPTIONS:");
    println!("    --fps <FPS>, -f <FPS>  Set frame rate (1-60, default: 10)");
    println!();
    println!("EXAMPLES:");
    println!("    fw16-pong-wars                 # Show help");
    println!("    fw16-pong-wars sim              # Run simulation at 10 FPS");
    println!("    fw16-pong-wars sim --fps 30     # Run simulation at 30 FPS");
    println!("    fw16-pong-wars live             # Run on LED Matrix");
    println!("    fw16-pong-wars check            # Check LED Matrix availability");
    println!();
    println!("NOTES:");
    println!("    - Live mode requires the Framework 16 LED Matrix module");
    println!("    - Live mode runs at hardware speed (~6 FPS for grayscale)");
    println!("    - Simulation mode allows adjustable frame rates");
    println!("    - Press Ctrl+C to exit the game");
}

fn run_live_mode() -> Result<()> {
    let mut led_matrix = LedMatrix::new()?;
    led_matrix.init_display()?;
    
    let force_exit = Arc::new(AtomicBool::new(false));
    let tray = SystemTray::new(force_exit.clone())?;
    
    let mut game_state = GameState::new();
    
    let frame_duration = Duration::from_secs_f64(1.0 / 60.0);
    let mut last_frame = Instant::now();
    let mut frame_count = 0u64;
    
    println!("Pong Wars is running in live mode...");
    println!("Hardware FPS: ~6 (firmware limited)");
    println!("Check system tray to exit.");
    
    loop {
        if let Some(TrayCommand::Exit) = tray.check_commands() {
            break;
        }
        
        if force_exit.load(Ordering::Relaxed) {
            break;
        }
        
        let now = Instant::now();
        if now.duration_since(last_frame) >= frame_duration {
            last_frame = now;
            frame_count += 1;
            
            game_state.update();
            
            led_matrix.render(&game_state)?;
            
            if frame_count % 10 == 0 {
                print!("\rDay: {} | Night: {} | Frame: {}    ", 
                    game_state.day_score, game_state.night_score, frame_count);
                io::stdout().flush()?;
            }
        }
        
        thread::sleep(Duration::from_millis(LED_SLEEP_MS));
    }
    
    println!("\nExiting live mode...");
    Ok(())
}

fn run_simulation_mode(fps: f64) -> Result<()> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        terminal::Clear(ClearType::All),
        cursor::Hide
    )?;
    
    let mut game_state = GameState::new();
    let mut previous_state = game_state.clone();
    
    let frame_duration = Duration::from_secs_f64(1.0 / fps);
    let display_duration = Duration::from_secs_f64(1.0 / DISPLAY_FPS.min(fps));
    
    let mut last_frame = Instant::now();
    let mut last_display = Instant::now();
    let mut frame_count = 0u64;
    
    let mut render_buffer = String::with_capacity(1024);
    
    render_game_state_buffered(&mut stdout, &game_state, &previous_state, fps, frame_count, true, &mut render_buffer)?;
    
    let result = loop {
        if crossterm::event::poll(Duration::from_millis(0))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                if key.code == crossterm::event::KeyCode::Char('c') &&
                   key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    break Ok(());
                }
            }
        }
        
        let now = Instant::now();
        if now.duration_since(last_frame) >= frame_duration {
            last_frame = now;
            frame_count += 1;
            
            game_state.update();
            
            if now.duration_since(last_display) >= display_duration {
                last_display = now;
                render_game_state_buffered(&mut stdout, &game_state, &previous_state, fps, frame_count, false, &mut render_buffer)?;
                previous_state = game_state.clone();
            }
        }
        
        thread::sleep(Duration::from_millis(1));
    };
    
    execute!(
        stdout,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )?;
    terminal::disable_raw_mode()?;
    
    result
}

fn render_game_state(
    stdout: &mut io::Stdout,
    game_state: &GameState,
    previous_state: &GameState,
    fps: f64,
    frame_count: u64,
    force_full_render: bool
) -> Result<()> {
    let mut buffer = String::with_capacity(1024);
    render_game_state_buffered(stdout, game_state, previous_state, fps, frame_count, force_full_render, &mut buffer)
}

fn render_game_state_buffered(
    stdout: &mut io::Stdout,
    game_state: &GameState,
    previous_state: &GameState,
    fps: f64,
    frame_count: u64,
    force_full_render: bool,
    buffer: &mut String
) -> Result<()> {
    buffer.clear();
    execute!(
        stdout,
        cursor::MoveTo(0, 0),
        Print("FW16 Pong Wars - Simulation Mode"),
        cursor::MoveTo(0, 1),
        Print(format!("Day: {} | Night: {} | FPS: {:.1} | Frame: {}    ", 
            game_state.day_score, game_state.night_score, fps, frame_count))
    )?;
    
    let grid_start_row = 3;
    
    for y in 0..game::GRID_HEIGHT {
        for x in 0..game::GRID_WIDTH {
            let current_has_ball = game_state.balls.iter()
                .any(|ball| ball.x as usize == x && ball.y as usize == y);
            let previous_has_ball = previous_state.balls.iter()
                .any(|ball| ball.x as usize == x && ball.y as usize == y);
            
            let current_color = game_state.squares[x][y];
            let previous_color = previous_state.squares[x][y];
            
            if force_full_render || current_has_ball != previous_has_ball || 
               current_color != previous_color {
                execute!(
                    stdout,
                    cursor::MoveTo(x as u16, (y + grid_start_row) as u16)
                )?;
                
                if current_has_ball {
                    execute!(
                        stdout,
                        SetForegroundColor(Color::Red),
                        Print("●"),
                        ResetColor
                    )?;
                } else {
                    let (color, symbol) = match current_color {
                        game::SquareColor::Day => (Color::White, "□"),
                        game::SquareColor::Night => (Color::Black, "■"),
                    };
                    execute!(
                        stdout,
                        SetForegroundColor(color),
                        Print(symbol),
                        ResetColor
                    )?;
                }
            }
        }
    }
    
    stdout.flush()?;
    Ok(())
}
