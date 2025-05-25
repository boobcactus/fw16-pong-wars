use rand::Rng;

pub const GRID_WIDTH: usize = 9;
pub const GRID_HEIGHT: usize = 34;
pub const TOTAL_SQUARES: usize = GRID_WIDTH * GRID_HEIGHT;

const MIN_SPEED: f32 = 0.2;
const MAX_SPEED: f32 = 0.5;
const SPEED_RANDOMNESS: f32 = 0.01;

// Pre-calculated constants
const GRID_WIDTH_F32: f32 = GRID_WIDTH as f32;
const GRID_HEIGHT_F32: f32 = GRID_HEIGHT as f32;
const HALF_GRID_HEIGHT: usize = GRID_HEIGHT / 2;
const GRID_HEIGHT_QUARTER: f32 = GRID_HEIGHT_F32 / 4.0;
const GRID_HEIGHT_THREE_QUARTERS: f32 = (GRID_HEIGHT_F32 * 3.0) / 4.0;
const GRID_WIDTH_HALF: f32 = GRID_WIDTH_F32 / 2.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SquareColor {
    Day,
    Night,
}

#[derive(Clone, Copy)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
    pub color_type: SquareColor,
}

impl Ball {
    #[inline]
    fn new(x: f32, y: f32, dx: f32, dy: f32, color_type: SquareColor) -> Self {
        Ball { x, y, dx, dy, color_type }
    }
    
    #[inline]
    fn update(&mut self, squares: &mut [[SquareColor; GRID_HEIGHT]; GRID_WIDTH], rng: &mut impl Rng) {
        // Check boundary collisions
        if self.x + self.dx >= GRID_WIDTH_F32 - 0.5 || self.x + self.dx < 0.5 {
            self.dx = -self.dx;
        }
        if self.y + self.dy >= GRID_HEIGHT_F32 - 0.5 || self.y + self.dy < 0.5 {
            self.dy = -self.dy;
        }
        
        // Check square collisions using static array
        const CHECK_OFFSETS: [(f32, f32); 4] = [(0.5, 0.0), (-0.5, 0.0), (0.0, 0.5), (0.0, -0.5)];
        
        for &(offset_x, offset_y) in &CHECK_OFFSETS {
            let check_x = self.x + offset_x;
            let check_y = self.y + offset_y;
            let grid_x = check_x as usize;
            let grid_y = check_y as usize;
            
            if grid_x < GRID_WIDTH && grid_y < GRID_HEIGHT {
                if squares[grid_x][grid_y] != self.color_type {
                    // Hit a square of opposite color
                    squares[grid_x][grid_y] = self.color_type;
                    
                    // Bounce
                    if offset_x.abs() > offset_y.abs() {
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
        self.dx += rng.gen_range(-SPEED_RANDOMNESS..SPEED_RANDOMNESS);
        self.dy += rng.gen_range(-SPEED_RANDOMNESS..SPEED_RANDOMNESS);
        
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

#[derive(Clone)]
pub struct GameState {
    pub squares: [[SquareColor; GRID_HEIGHT]; GRID_WIDTH],
    pub balls: [Ball; 2], // Fixed size array for 2 balls
    pub day_score: usize,
    pub night_score: usize,
    rng: rand::rngs::ThreadRng, // Store RNG to avoid repeated allocations
}

impl GameState {
    pub fn new() -> Self {
        let mut squares = [[SquareColor::Day; GRID_HEIGHT]; GRID_WIDTH];
        
        // Initialize field - top half night, bottom half day
        for x in 0..GRID_WIDTH {
            for y in 0..HALF_GRID_HEIGHT {
                squares[x][y] = SquareColor::Night;
            }
            // Bottom half is already Day from initialization
        }
        
        // Initialize balls - one in each half
        let balls = [
            Ball::new(
                GRID_WIDTH_HALF,
                GRID_HEIGHT_THREE_QUARTERS,  // Bottom quarter (Day territory)
                0.3,
                -0.3,
                SquareColor::Day,
            ),
            Ball::new(
                GRID_WIDTH_HALF,
                GRID_HEIGHT_QUARTER,  // Top quarter (Night territory)
                -0.3,
                0.3,
                SquareColor::Night,
            ),
        ];
        
        // Count initial scores
        let day_score = HALF_GRID_HEIGHT * GRID_WIDTH;
        let night_score = HALF_GRID_HEIGHT * GRID_WIDTH;
        
        GameState {
            squares,
            balls,
            day_score,
            night_score,
            rng: rand::thread_rng(),
        }
    }
    
    #[inline]
    pub fn update(&mut self) {
        // Update balls
        for ball in &mut self.balls {
            ball.update(&mut self.squares, &mut self.rng);
        }
        
        // Count scores - optimize by tracking deltas in the future
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
