use anyhow::{anyhow, Result};
use serialport::{DataBits, Parity, SerialPort, StopBits};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::game::{GameState, SquareColor};

const BAUD_RATE: u32 = 115200;
const TIMEOUT_MS: u64 = 5000;

// Framework LED Matrix Protocol Constants
const MAGIC_WORD: [u8; 2] = [0x32, 0xAC];

// Command IDs
const CMD_BRIGHTNESS: u8 = 0x00;
const CMD_DRAW_BW: u8 = 0x06;
const CMD_STAGE_GREY_COL: u8 = 0x07;
const CMD_DRAW_GREY_BUFFER: u8 = 0x08;

// Pre-calculated buffer sizes
const COMMIT_CMD_SIZE: usize = 4; // Magic(2) + Cmd(1) + Unused(1)
const MODULE_WIDTH: usize = 9;

// Flow control constants
const RECOVERY_DELAY_MS: u64 = 2000; // Delay after error before retry
const MAX_CONSECUTIVE_ERRORS: u32 = 3; // Max errors before reset attempt

struct MatrixPort {
    port: Box<dyn SerialPort>,
    #[allow(dead_code)]
    column_buffer: Vec<u8>,
    #[allow(dead_code)]
    commit_buffer: [u8; COMMIT_CMD_SIZE],
    width: usize,
    #[allow(dead_code)]
    last_columns: Vec<Vec<u8>>,
}

impl MatrixPort {
    #[allow(unused_mut)]
    fn new(mut port: Box<dyn SerialPort>, height: usize) -> Result<Self> {
        if let Err(e) = port.clear(serialport::ClearBuffer::All) {
            return Err(anyhow!("Failed clearing port: {}", e));
        }
        thread::sleep(Duration::from_millis(100));

        let mut column_buffer = Vec::with_capacity(4 + height);
        column_buffer.extend_from_slice(&MAGIC_WORD);
        column_buffer.push(CMD_STAGE_GREY_COL);
        column_buffer.push(0);
        column_buffer.resize(4 + height, 0);

        let commit_buffer = [MAGIC_WORD[0], MAGIC_WORD[1], CMD_DRAW_GREY_BUFFER, 0x00];

        Ok(MatrixPort {
            port,
            column_buffer,
            commit_buffer,
            width: MODULE_WIDTH,
            last_columns: vec![vec![0xEE; height]; MODULE_WIDTH],
        })
    }
}

pub struct LedMatrix {
    ports: Vec<MatrixPort>,
    brightness: Arc<AtomicU8>,
    consecutive_errors: u32,
    width: usize,
    height: usize,
}

impl LedMatrix {
    pub fn new_with_brightness(brightness: Arc<AtomicU8>, dual_mode: bool, height: usize) -> Result<Self> {
        let mut candidates: Vec<serialport::SerialPortInfo> = serialport::available_ports()?
            .into_iter()
            .filter(|p| matches!(p.port_type, serialport::SerialPortType::UsbPort(ref info) if info.vid == 0x32AC && (info.pid == 0x0020 || info.pid == 0x0021)))
            .collect();

        if candidates.is_empty() {
            return Err(anyhow!("No Framework LED Matrix modules found."));
        }

        candidates.sort_by(|a, b| {
            let sa = match &a.port_type {
                serialport::SerialPortType::UsbPort(info) => info.serial_number.as_deref(),
                _ => None,
            };
            let sb = match &b.port_type {
                serialport::SerialPortType::UsbPort(info) => info.serial_number.as_deref(),
                _ => None,
            };
            match (sa, sb) {
                (Some(aa), Some(bb)) => aa.cmp(bb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.port_name.cmp(&b.port_name),
            }
        });

        let mut desired_ports = if dual_mode {
            if candidates.len() < 2 {
                return Err(anyhow!("Dual mode requested but only {} LED Matrix module detected.", candidates.len()));
            }
            candidates.truncate(2);
            candidates
        } else {
            candidates.truncate(1);
            candidates
        };

        if dual_mode && desired_ports.len() == 2 {
            desired_ports.reverse();
            println!(
                "Auto-ordered modules: {} = right, {} = left",
                desired_ports[0].port_name, desired_ports[1].port_name
            );
        }

        let mut matrix_ports: Vec<MatrixPort> = Vec::new();
        for info in desired_ports {
            match serialport::new(&info.port_name, BAUD_RATE)
                .timeout(Duration::from_millis(TIMEOUT_MS))
                .data_bits(DataBits::Eight)
                .parity(Parity::None)
                .stop_bits(StopBits::One)
                .open()
            {
                Ok(port) => match MatrixPort::new(Box::from(port), height) {
                    Ok(matrix_port) => {
                        println!("Connected LED Matrix on {}", info.port_name);
                        matrix_ports.push(matrix_port);
                    }
                    Err(e) => eprintln!("Failed initializing port {}: {}", info.port_name, e),
                },
                Err(e) => eprintln!("Failed opening port {}: {}", info.port_name, e),
            }
        }

        if matrix_ports.is_empty() {
            return Err(anyhow!("Unable to open any Framework LED Matrix modules."));
        }

        Ok(LedMatrix {
            width: matrix_ports.len() * MODULE_WIDTH,
            height,
            ports: matrix_ports,
            brightness,
            consecutive_errors: 0,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn reconnect(&mut self) -> Result<()> {
        println!("Attempting to reconnect to LED Matrix...");
        
        thread::sleep(Duration::from_millis(RECOVERY_DELAY_MS));

        let dual_mode = self.ports.len() > 1;
        let brightness = self.brightness.clone();
        let new_self = Self::new_with_brightness(brightness, dual_mode, self.height)?;

        *self = new_self;

        println!("Successfully reconnected to LED Matrix");
        Ok(())
    }

    #[inline]
    pub fn set_brightness(&mut self, brightness: u8) -> Result<()> {
        self.brightness.store(brightness, Ordering::SeqCst);
        for (idx, port) in self.ports.iter_mut().enumerate() {
            let buf = [MAGIC_WORD[0], MAGIC_WORD[1], CMD_BRIGHTNESS, brightness];
            port
                .port
                .write_all(&buf)
                .map_err(|e| anyhow!("Failed to set brightness on port {}: {}", idx, e))?;
        }
        Ok(())
    }

    #[inline]
    pub fn render(&mut self, game_state: &GameState) -> Result<()> {
        // Attempt render with error recovery
        match self.render_internal(game_state) {
            Ok(()) => {
                self.consecutive_errors = 0;
                Ok(())
            }
            Err(e) => {
                self.consecutive_errors += 1;
                eprintln!(
                    "Render error (#{} consecutive): {}",
                    self.consecutive_errors, e
                );

                // If too many consecutive errors, attempt reconnection
                if self.consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                    eprintln!("Too many consecutive errors, attempting device reset...");
                    match self.reconnect() {
                        Ok(()) => {
                            // Try rendering again after successful reconnection
                            self.render_internal(game_state)
                        }
                        Err(reconnect_err) => {
                            eprintln!("Failed to reconnect: {}", reconnect_err);
                            Err(reconnect_err)
                        }
                    }
                } else {
                    // For occasional errors, just wait a bit and continue
                    thread::sleep(Duration::from_millis(50));
                    Err(e)
                }
            }
        }
    }

    #[inline]
    fn render_internal(&mut self, game_state: &GameState) -> Result<()> {
        for port_index in 0..self.ports.len() {
            let port = &mut self.ports[port_index];

            let mut vals = [0u8; 39];
            for y in 0..self.height {
                if y >= game_state.height() {
                    break;
                }
                for local_x in 0..port.width {
                    let global_x = port_index * port.width + local_x;
                    if global_x >= game_state.width() {
                        break;
                    }

                    let square_color = game_state.squares[global_x][y];
                    let has_ball = game_state
                        .balls
                        .iter()
                        .any(|ball| (ball.x as usize == global_x) && (ball.y as usize == y));

                    let on = match square_color {
                        SquareColor::Day => !has_ball,
                        SquareColor::Night => has_ball,
                    };

                    if on {
                        let i = local_x + MODULE_WIDTH * y;
                        let byte = i / 8;
                        let bit = i % 8;
                        vals[byte] |= 1u8 << bit;
                    }
                }
            }

            let mut buf = Vec::with_capacity(3 + vals.len());
            buf.push(MAGIC_WORD[0]);
            buf.push(MAGIC_WORD[1]);
            buf.push(CMD_DRAW_BW);
            buf.extend_from_slice(&vals);
            port
                .port
                .write_all(&buf)
                .map_err(|e| anyhow!("Failed to write BW frame on port {}: {}", port_index, e))?;
        }

        Ok(())
    }

    pub fn estimated_max_fps(&self) -> u32 {
        // Using DrawBW (0x06): 2 magic + 1 cmd + 39 payload = 42 bytes per port per frame
        let per_port = 2 + 1 + 39;
        let total = self.ports.len() * per_port;
        let bytes_per_sec = (BAUD_RATE as f64) / 10.0;
        let fps = (bytes_per_sec / ((total as f64) * 1.1)).floor() as u32;
        if fps < 1 { 1 } else { fps }
    }
}
