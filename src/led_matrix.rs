use anyhow::{anyhow, Result};
use serialport::{DataBits, Parity, SerialPort, StopBits};
use std::time::Duration;
use std::thread;

use crate::game::{GameState, SquareColor, GRID_HEIGHT, GRID_WIDTH};

const BAUD_RATE: u32 = 115200;
const TIMEOUT_MS: u64 = 5000; // Increased timeout to 5 seconds

// Framework LED Matrix Protocol Constants
const MAGIC_WORD: [u8; 2] = [0x32, 0xAC];

// Command IDs from commands.md
const CMD_BRIGHTNESS: u8 = 0x00;
const CMD_STAGE_GREY_COL: u8 = 0x07;
const CMD_DRAW_GREY_BUFFER: u8 = 0x08;

pub struct LedMatrix {
    port: Box<dyn SerialPort>,
}

impl LedMatrix {
    /// Check if the Framework LED Matrix is available without initializing it
    pub fn check_available() -> Result<()> {
        let ports = serialport::available_ports()?;
        
        ports
            .iter()
            .find(|p| {
                if let serialport::SerialPortType::UsbPort(info) = &p.port_type {
                    // Framework Computer vendor ID is 0x32AC
                    // LED Matrix product ID is typically 0x0020 or 0x0021
                    info.vid == 0x32AC && (info.pid == 0x0020 || info.pid == 0x0021)
                } else {
                    false
                }
            })
            .ok_or_else(|| anyhow!("Framework LED Matrix not found."))?;
        
        Ok(())
    }
    
    /// Find all Framework LED Matrix devices and return detailed information
    pub fn find_all_devices() -> Result<Vec<(serialport::SerialPortInfo, String)>> {
        let ports = serialport::available_ports()?;
        let mut devices = Vec::new();
        
        for port in ports {
            if let serialport::SerialPortType::UsbPort(info) = &port.port_type {
                // Framework Computer vendor ID is 0x32AC
                // LED Matrix product ID is typically 0x0020 or 0x0021
                if info.vid == 0x32AC && (info.pid == 0x0020 || info.pid == 0x0021) {
                    let device_info = format!(
                        "VID: 0x{:04X}, PID: 0x{:04X}, Serial: {}, Manufacturer: {}, Product: {}",
                        info.vid,
                        info.pid,
                        info.serial_number.as_deref().unwrap_or("N/A"),
                        info.manufacturer.as_deref().unwrap_or("N/A"),
                        info.product.as_deref().unwrap_or("N/A")
                    );
                    devices.push((port.clone(), device_info));
                }
            }
        }
        
        Ok(devices)
    }
    
    pub fn new() -> Result<Self> {
        // Find the Framework LED Matrix port
        let ports = serialport::available_ports()?;
        
        let framework_port = ports
            .iter()
            .find(|p| {
                if let serialport::SerialPortType::UsbPort(info) = &p.port_type {
                    // Framework Computer vendor ID is 0x32AC
                    // LED Matrix product ID is typically 0x0020 or 0x0021
                    info.vid == 0x32AC && (info.pid == 0x0020 || info.pid == 0x0021)
                } else {
                    false
                }
            })
            .ok_or_else(|| anyhow!("Framework LED Matrix not found. Please ensure the module is connected."))?;
        
        println!("Found Framework LED Matrix on port: {}", framework_port.port_name);
        
        // Open the port
        let port = serialport::new(&framework_port.port_name, BAUD_RATE)
            .timeout(Duration::from_millis(TIMEOUT_MS))
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .open()?;
        
        // Clear input/output buffers
        port.clear(serialport::ClearBuffer::All)?;
        
        // Small delay to let the device settle
        thread::sleep(Duration::from_millis(100));
        
        Ok(LedMatrix { port: Box::from(port) })
    }
    
    pub fn init_display(&mut self) -> Result<()> {
        // Send initialization sequence
        // Clear display first
        self.clear_display()?;
        
        // Set brightness to medium
        self.set_brightness(128)?;
        
        Ok(())
    }
    
    fn clear_display(&mut self) -> Result<()> {
        // Send all columns with brightness 0
        for x in 0..9 {
            let mut column_data = vec![
                MAGIC_WORD[0], MAGIC_WORD[1],
                CMD_STAGE_GREY_COL,
                x as u8,
            ];
            
            // Add 34 zeros for this column
            column_data.extend(vec![0u8; 34]);
            
            self.port.write_all(&column_data)?;
            self.port.flush()?;
            
            // Small delay between columns to avoid overwhelming the device
            thread::sleep(Duration::from_millis(5));
        }
        
        // Commit to display
        let commit_command = vec![
            MAGIC_WORD[0], MAGIC_WORD[1],
            CMD_DRAW_GREY_BUFFER,
            0x00,
        ];
        
        self.port.write_all(&commit_command)?;
        self.port.flush()?;
        
        // Wait for display to update
        thread::sleep(Duration::from_millis(50));
        
        Ok(())
    }
    
    fn set_brightness(&mut self, brightness: u8) -> Result<()> {
        // Command: Set brightness (0-255)
        let command = vec![
            MAGIC_WORD[0], MAGIC_WORD[1],
            CMD_BRIGHTNESS,
            brightness,
        ];
        
        self.port.write_all(&command)?;
        self.port.flush()?;
        
        // Small delay to let command process
        thread::sleep(Duration::from_millis(10));
        
        Ok(())
    }
    
    pub fn render(&mut self, game_state: &GameState) -> Result<()> {
        // Send each column of grayscale data
        for x in 0..GRID_WIDTH {
            let mut column_data = vec![
                MAGIC_WORD[0], MAGIC_WORD[1],
                CMD_STAGE_GREY_COL,
                x as u8,  // Column index
            ];
            
            // Add 34 brightness values for this column
            for y in 0..GRID_HEIGHT {
                // Check if ball is at this position
                let mut has_ball = false;
                for ball in &game_state.balls {
                    if (ball.x as usize == x) && (ball.y as usize == y) {
                        has_ball = true;
                        break;
                    }
                }
                
                let brightness = if has_ball {
                    // Ball brightness depends on the square it's on
                    match game_state.squares[x][y] {
                        SquareColor::Day => 0,      // Dark ball on bright square
                        SquareColor::Night => 255,   // Bright ball on dark square
                    }
                } else {
                    match game_state.squares[x][y] {
                        SquareColor::Day => 200,    // Bright for day
                        SquareColor::Night => 0,     // Off for night
                    }
                };
                
                column_data.push(brightness);
            }
            
            // Send this column
            self.port.write_all(&column_data)?;
            self.port.flush()?;
            
            // Small delay between columns
            thread::sleep(Duration::from_millis(2));
        }
        
        // Commit all columns to display
        let commit_command = vec![
            MAGIC_WORD[0], MAGIC_WORD[1],
            CMD_DRAW_GREY_BUFFER,
            0x00,  // Unused parameter
        ];
        
        self.port.write_all(&commit_command)?;
        self.port.flush()?;
        
        Ok(())
    }
}
