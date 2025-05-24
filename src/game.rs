use rand::Rng;

pub const GRID_WIDTH: usize = 9;
pub const GRID_HEIGHT: usize = 34;

const MIN_SPEED: f32 = 0.2;
const MAX_SPEED: f32 = 0.5;

#[derive(Clone, Copy, PartialEq)]
pub enum SquareColor {
    Day,
    Night,
}

pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
    pub color_type: SquareColor,
}

impl Ball {
    fn new(x: f32, y: f32, dx: f32, dy: f32, color_type: SquareColor) -> Self {
        Ball {
            x,
            y,
            dx,
            dy,
            color_type,
        }
    }
    
    fn update(&mut self, squares: &mut [[SquareColor; GRID_HEIGHT]; GRID_WIDTH]) {
        // Check boundary collisions
        if self.x + self.dx >= GRID_WIDTH as f32 - 0.5 || self.x + self.dx < 0.5 {
            self.dx = -self.dx;
        }
        if self.y + self.dy >= GRID_HEIGHT as f32 - 0.5 || self.y + self.dy < 0.5 {
            self.dy = -self.dy;
        }
        
        // Check square collisions
        let check_positions = [
            (self.x + 0.5, self.y),
            (self.x - 0.5, self.y),
            (self.x, self.y + 0.5),
            (self.x, self.y - 0.5),
        ];
        
        for (check_x, check_y) in check_positions {
            let grid_x = check_x as usize;
            let grid_y = check_y as usize;
            
            if grid_x < GRID_WIDTH && grid_y < GRID_HEIGHT {
                if squares[grid_x][grid_y] != self.color_type {
                    // Hit a square of opposite color
                    squares[grid_x][grid_y] = self.color_type;
                    
                    // Bounce
                    if (check_x - self.x).abs() > (check_y - self.y).abs() {
                        self.dx = -self.dx;
                    } else {
                        self.dy = -self.dy;
                    }
                }
            }
        }
        
        // Update position
        self.x += self.dx;
        self.y += self.dy;
        
        // Add randomness
        let mut rng = rand::thread_rng();
        self.dx += rng.gen_range(-0.01..0.01);
        self.dy += rng.gen_range(-0.01..0.01);
        
        // Clamp speed
        self.dx = self.dx.clamp(-MAX_SPEED, MAX_SPEED);
        self.dy = self.dy.clamp(-MAX_SPEED, MAX_SPEED);
        
        // Ensure minimum speed
        if self.dx.abs() < MIN_SPEED {
            self.dx = if self.dx > 0.0 { MIN_SPEED } else { -MIN_SPEED };
        }
        if self.dy.abs() < MIN_SPEED {
            self.dy = if self.dy > 0.0 { MIN_SPEED } else { -MIN_SPEED };
        }
    }
}

pub struct GameState {
    pub squares: [[SquareColor; GRID_HEIGHT]; GRID_WIDTH],
    pub balls: Vec<Ball>,
    pub day_score: usize,
    pub night_score: usize,
}

impl GameState {
    pub fn new() -> Self {
        let mut squares = [[SquareColor::Day; GRID_HEIGHT]; GRID_WIDTH];
        
        // Initialize field - top half night, bottom half day (split horizontally)
        for x in 0..GRID_WIDTH {
            for y in 0..GRID_HEIGHT {
                squares[x][y] = if y < GRID_HEIGHT / 2 {
                    SquareColor::Night
                } else {
                    SquareColor::Day
                };
            }
        }
        
        // Initialize balls - one in each half
        let balls = vec![
            Ball::new(
                GRID_WIDTH as f32 / 2.0,
                (GRID_HEIGHT as f32 * 3.0) / 4.0,  // Bottom quarter (Day territory)
                0.3,
                -0.3,
                SquareColor::Day,
            ),
            Ball::new(
                GRID_WIDTH as f32 / 2.0,
                GRID_HEIGHT as f32 / 4.0,  // Top quarter (Night territory)
                -0.3,
                0.3,
                SquareColor::Night,
            ),
        ];
        
        GameState {
            squares,
            balls,
            day_score: 0,
            night_score: 0,
        }
    }
    
    pub fn update(&mut self) {
        // Update balls
        for ball in &mut self.balls {
            ball.update(&mut self.squares);
        }
        
        // Count scores
        self.day_score = 0;
        self.night_score = 0;
        
        for x in 0..GRID_WIDTH {
            for y in 0..GRID_HEIGHT {
                match self.squares[x][y] {
                    SquareColor::Day => self.day_score += 1,
                    SquareColor::Night => self.night_score += 1,
                }
            }
        }
    }
}
