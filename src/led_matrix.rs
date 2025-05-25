use anyhow::{anyhow, Result};
use serialport::{DataBits, Parity, SerialPort, StopBits};
use std::time::Duration;
use std::thread;

use crate::game::{GameState, SquareColor, GRID_HEIGHT, GRID_WIDTH};

const BAUD_RATE: u32 = 115200;
const TIMEOUT_MS: u64 = 5000;

// Framework LED Matrix Protocol Constants
const MAGIC_WORD: [u8; 2] = [0x32, 0xAC];

// Command IDs
const CMD_BRIGHTNESS: u8 = 0x00;
const CMD_STAGE_GREY_COL: u8 = 0x07;
const CMD_DRAW_GREY_BUFFER: u8 = 0x08;

// Pre-calculated buffer sizes
const COLUMN_DATA_SIZE: usize = 4 + GRID_HEIGHT; // Magic(2) + Cmd(1) + ColIdx(1) + Data(34)
const COMMIT_CMD_SIZE: usize = 4; // Magic(2) + Cmd(1) + Unused(1)

pub struct LedMatrix {
    port: Box<dyn SerialPort>,
    column_buffer: Vec<u8>, // Pre-allocated buffer for column data
    commit_buffer: [u8; COMMIT_CMD_SIZE], // Static buffer for commit command
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
        
        // Pre-allocate buffers
        let mut column_buffer = Vec::with_capacity(COLUMN_DATA_SIZE);
        column_buffer.extend_from_slice(&MAGIC_WORD);
        column_buffer.push(CMD_STAGE_GREY_COL);
        column_buffer.push(0); // Column index placeholder
        column_buffer.resize(COLUMN_DATA_SIZE, 0); // Fill with zeros
        
        let commit_buffer = [
            MAGIC_WORD[0], 
            MAGIC_WORD[1],
            CMD_DRAW_GREY_BUFFER,
            0x00,
        ];
        
        Ok(LedMatrix { 
            port: Box::from(port),
            column_buffer,
            commit_buffer,
        })
    }
    
    pub fn init_display(&mut self) -> Result<()> {
        // Send initialization sequence
        self.clear_display()?;
        self.set_brightness(128)?;
        Ok(())
    }
    
    #[inline]
    fn clear_display(&mut self) -> Result<()> {
        // Use pre-allocated buffer
        self.column_buffer[3] = 0; // Start with column 0
        
        // Clear all brightness values
        for i in 4..COLUMN_DATA_SIZE {
            self.column_buffer[i] = 0;
        }
        
        // Send all columns with brightness 0
        for x in 0..9 {
            self.column_buffer[3] = x as u8;
            self.port.write_all(&self.column_buffer)?;
        }
        
        self.port.flush()?;
        
        // Commit to display
        self.port.write_all(&self.commit_buffer)?;
        self.port.flush()?;
        
        Ok(())
    }
    
    #[inline]
    fn set_brightness(&mut self, brightness: u8) -> Result<()> {
        // Command: Set brightness (0-255)
        const BRIGHTNESS_CMD: [u8; 4] = [
            MAGIC_WORD[0], 
            MAGIC_WORD[1],
            CMD_BRIGHTNESS,
            0, // Brightness placeholder
        ];
        
        let mut cmd = BRIGHTNESS_CMD;
        cmd[3] = brightness;
        
        self.port.write_all(&cmd)?;
        self.port.flush()?;
        
        Ok(())
    }
    
    #[inline]
    pub fn render(&mut self, game_state: &GameState) -> Result<()> {
        // Send each column of grayscale data
        for x in 0..GRID_WIDTH {
            self.column_buffer[3] = x as u8;  // Column index
            
            // Add 34 brightness values for this column
            for y in 0..GRID_HEIGHT {
                // Check if ball is at this position
                let has_ball = game_state.balls.iter()
                    .any(|ball| (ball.x as usize == x) && (ball.y as usize == y));
                
                let brightness = if has_ball {
                    // Ball brightness depends on the square it's on
                    match game_state.squares[x][y] {
                        SquareColor::Day => 0,      // Dark ball on bright square
                        SquareColor::Night => 255,   // Bright ball on dark square
                    }
                } else {
                    match game_state.squares[x][y] {
                        SquareColor::Day => 255,    // Full bright for day (white)
                        SquareColor::Night => 0,     // Full off for night (black)
                    }
                };
                
                self.column_buffer[4 + y] = brightness;
            }
            
            // Send this column
            self.port.write_all(&self.column_buffer)?;
        }
        
        // Flush once after all columns
        self.port.flush()?;
        
        // Commit all columns to display
        self.port.write_all(&self.commit_buffer)?;
        self.port.flush()?;
        
        Ok(())
    }
}
