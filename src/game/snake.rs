use image::{Rgb, RgbImage};
use rusttype::Font;
use std::collections::{HashSet, VecDeque};
use std::time::Instant;

use crate::device::{BUTTON_HEIGHT, BUTTON_WIDTH, STRIP_HEIGHT, STRIP_WIDTH};
use crate::display::renderer::{draw_filled_rect, draw_text, text_width};

// Grid dimensions: 35 columns x 14 rows across 10 buttons (5x2)
const GRID_COLS: usize = 35;
const GRID_ROWS: usize = 14;
const CELL_PITCH: u32 = 16; // 16px per cell = 7 cells per 112px button
const CELL_SIZE: u32 = 14; // 14px filled area + 2px grid gap
const CELLS_PER_BUTTON: usize = 7;

// Colors
const BG_COLOR: Rgb<u8> = Rgb([8, 10, 16]);
const GRID_COLOR: Rgb<u8> = Rgb([20, 22, 30]);
const WALL_HINT: Rgb<u8> = Rgb([30, 32, 42]);
const HEAD_COLOR: Rgb<u8> = Rgb([50, 220, 100]);
const BODY_BRIGHT: Rgb<u8> = Rgb([40, 180, 80]);
const BODY_DARK: Rgb<u8> = Rgb([20, 100, 50]);
const FOOD_COLOR_A: Rgb<u8> = Rgb([200, 50, 50]);
const FOOD_COLOR_B: Rgb<u8> = Rgb([255, 80, 80]);
const TITLE_COLOR_DIM: Rgb<u8> = Rgb([20, 100, 50]);
const TITLE_COLOR_BRIGHT: Rgb<u8> = Rgb([50, 220, 100]);
const DEATH_FLASH: Rgb<u8> = Rgb([200, 40, 40]);
const STRIP_BG: Rgb<u8> = Rgb([12, 14, 20]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn turn_left(self) -> Direction {
        match self {
            Direction::Up => Direction::Left,
            Direction::Left => Direction::Down,
            Direction::Down => Direction::Right,
            Direction::Right => Direction::Up,
        }
    }

    pub fn turn_right(self) -> Direction {
        match self {
            Direction::Up => Direction::Right,
            Direction::Right => Direction::Down,
            Direction::Down => Direction::Left,
            Direction::Left => Direction::Up,
        }
    }

    pub fn delta(self) -> (i32, i32) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Direction::Up => "UP",
            Direction::Down => "DOWN",
            Direction::Left => "LEFT",
            Direction::Right => "RIGHT",
        }
    }

    pub fn arrow(self) -> &'static str {
        match self {
            Direction::Up => "^",
            Direction::Down => "v",
            Direction::Left => "<",
            Direction::Right => ">",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Title,
    Playing,
    GameOver,
}

/// Simple LCG random number generator (no external crate needed)
struct Rng {
    state: u64,
}

impl Rng {
    fn new() -> Self {
        // Seed from current time nanos
        let seed = Instant::now().elapsed().as_nanos() as u64;
        // Mix in some entropy from the address of a stack variable
        let extra = &seed as *const u64 as u64;
        Self {
            state: seed.wrapping_add(extra).wrapping_add(12345),
        }
    }

    fn next(&mut self) -> u64 {
        // LCG: state = state * 6364136223846793005 + 1442695040888963407
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn next_range(&mut self, max: usize) -> usize {
        (self.next() % max as u64) as usize
    }
}

pub struct SnakeGame {
    snake: VecDeque<Pos>,
    /// O(1) collision lookup - mirrors snake contents
    body_set: HashSet<Pos>,
    direction: Direction,
    /// Queue of pending turns (up to 2 buffered, to handle fast input)
    turn_queue: VecDeque<Direction>,
    food: Pos,
    score: u32,
    high_score: u32,
    state: GameState,
    rng: Rng,

    // Timing
    last_move: Instant,
    move_interval_ms: u64,
    last_food_pulse: Instant,
    food_pulse_on: bool,
    last_blink: Instant,
    blink_on: bool,

    // Death animation
    death_frame: u8,
    last_death_frame: Instant,

    // Dirty tracking
    dirty_buttons: [bool; 10],
    strip_dirty: bool,
}

impl Default for SnakeGame {
    fn default() -> Self {
        Self::new()
    }
}

impl SnakeGame {
    pub fn new() -> Self {
        let now = Instant::now();
        let mut game = Self {
            snake: VecDeque::new(),
            body_set: HashSet::with_capacity(GRID_COLS * GRID_ROWS),
            direction: Direction::Right,
            turn_queue: VecDeque::with_capacity(4),
            food: Pos { x: 0, y: 0 },
            score: 0,
            high_score: 0,
            state: GameState::Title,
            rng: Rng::new(),
            last_move: now,
            move_interval_ms: 250,
            last_food_pulse: now,
            food_pulse_on: true,
            last_blink: now,
            blink_on: true,
            death_frame: 0,
            last_death_frame: now,
            dirty_buttons: [true; 10],
            strip_dirty: true,
        };
        game.setup_title();
        game
    }

    fn setup_title(&mut self) {
        self.state = GameState::Title;
        self.mark_all_dirty();
    }

    fn start_playing(&mut self) {
        self.snake.clear();
        self.body_set.clear();
        // Start snake in the center, 3 segments long
        let start_x = GRID_COLS as i32 / 2;
        let start_y = GRID_ROWS as i32 / 2;
        let segments = [
            Pos { x: start_x, y: start_y },
            Pos { x: start_x - 1, y: start_y },
            Pos { x: start_x - 2, y: start_y },
        ];
        for &seg in &segments {
            self.snake.push_back(seg);
            self.body_set.insert(seg);
        }

        self.direction = Direction::Right;
        self.turn_queue.clear();
        self.score = 0;
        self.move_interval_ms = 250;
        self.state = GameState::Playing;

        let now = Instant::now();
        self.last_move = now;
        self.last_food_pulse = now;

        self.spawn_food();
        self.mark_all_dirty();
    }

    fn spawn_food(&mut self) {
        loop {
            let x = self.rng.next_range(GRID_COLS) as i32;
            let y = self.rng.next_range(GRID_ROWS) as i32;
            let pos = Pos { x, y };
            if !self.body_set.contains(&pos) {
                self.food = pos;
                self.mark_button_for_cell(x, y);
                return;
            }
        }
    }

    /// Called at ~16ms intervals from the main loop.
    /// Updates game state (movement, animations) and sets dirty flags.
    pub fn tick(&mut self) {
        match self.state {
            GameState::Title => {
                // Blink "SNAKE" title every 500ms
                if self.last_blink.elapsed().as_millis() >= 500 {
                    self.last_blink = Instant::now();
                    self.blink_on = !self.blink_on;
                    self.mark_all_dirty();
                }
            }
            GameState::Playing => {
                // Food pulse animation (300ms)
                if self.last_food_pulse.elapsed().as_millis() >= 300 {
                    self.last_food_pulse = Instant::now();
                    self.food_pulse_on = !self.food_pulse_on;
                    self.mark_button_for_cell(self.food.x, self.food.y);
                }

                // Snake movement tick
                if self.last_move.elapsed().as_millis() >= self.move_interval_ms as u128 {
                    self.last_move = Instant::now();
                    self.advance_snake();
                }
            }
            GameState::GameOver => {
                if self.death_frame < 10 {
                    // Death animation: 80ms per frame
                    if self.last_death_frame.elapsed().as_millis() >= 80 {
                        self.last_death_frame = Instant::now();
                        self.death_frame += 1;
                        self.mark_all_dirty();
                    }
                } else {
                    // After death animation, blink game over text
                    if self.last_blink.elapsed().as_millis() >= 500 {
                        self.last_blink = Instant::now();
                        self.blink_on = !self.blink_on;
                        self.strip_dirty = true;
                    }
                }
            }
        }
    }

    fn advance_snake(&mut self) {
        // Apply next queued turn
        if let Some(new_dir) = self.turn_queue.pop_front() {
            self.direction = new_dir;
        }

        let (dx, dy) = self.direction.delta();
        let head = self.snake.front().unwrap();
        let new_head = Pos {
            x: head.x + dx,
            y: head.y + dy,
        };

        // Check wall collision
        if new_head.x < 0
            || new_head.x >= GRID_COLS as i32
            || new_head.y < 0
            || new_head.y >= GRID_ROWS as i32
        {
            self.die();
            return;
        }

        // Check self collision (O(1) with HashSet)
        if self.body_set.contains(&new_head) {
            self.die();
            return;
        }

        // Mark old head button dirty (color changes from head to body)
        self.mark_button_for_cell(head.x, head.y);
        // Mark new head button dirty
        self.mark_button_for_cell(new_head.x, new_head.y);

        self.snake.push_front(new_head);
        self.body_set.insert(new_head);

        if new_head == self.food {
            // Ate food - don't remove tail (snake grows)
            self.score += 1;
            if self.score > self.high_score {
                self.high_score = self.score;
            }
            // Speed up: decrease interval by 10ms per food, min 80ms
            if self.move_interval_ms > 80 {
                self.move_interval_ms = (self.move_interval_ms - 10).max(80);
            }
            self.strip_dirty = true;
            self.spawn_food();
        } else {
            // Remove tail
            if let Some(tail) = self.snake.pop_back() {
                self.body_set.remove(&tail);
                self.mark_button_for_cell(tail.x, tail.y);
            }
        }
    }

    fn die(&mut self) {
        self.state = GameState::GameOver;
        self.death_frame = 0;
        self.last_death_frame = Instant::now();
        self.last_blink = Instant::now();
        self.blink_on = true;
        self.mark_all_dirty();
    }

    // --- Input handling ---

    pub fn handle_encoder_rotate(&mut self, _encoder: u8, direction: i8) {
        if self.state != GameState::Playing {
            return;
        }
        // Buffer up to 3 turns so fast rotations aren't lost
        if self.turn_queue.len() < 3 {
            // Base the turn on the last queued direction (or current)
            let base_dir = self.turn_queue.back().copied().unwrap_or(self.direction);
            let new_dir = if direction < 0 {
                base_dir.turn_left()
            } else {
                base_dir.turn_right()
            };
            self.turn_queue.push_back(new_dir);
            // Mark strip dirty for immediate HUD feedback
            self.strip_dirty = true;
        }
    }

    /// The effective next direction (accounts for queued turns)
    pub fn effective_direction(&self) -> Direction {
        self.turn_queue.back().copied().unwrap_or(self.direction)
    }

    pub fn handle_button_press(&mut self, _button_id: u8) {
        match self.state {
            GameState::Title | GameState::GameOver => {
                self.start_playing();
            }
            GameState::Playing => {
                // No action during gameplay
            }
        }
    }

    pub fn game_state(&self) -> GameState {
        self.state
    }

    // --- Dirty tracking ---

    fn mark_all_dirty(&mut self) {
        self.dirty_buttons = [true; 10];
        self.strip_dirty = true;
    }

    fn mark_button_for_cell(&mut self, cx: i32, cy: i32) {
        let bx = cx as usize / CELLS_PER_BUTTON;
        let by = cy as usize / CELLS_PER_BUTTON;
        if bx < 5 && by < 2 {
            let button_id = by * 5 + bx;
            self.dirty_buttons[button_id] = true;
        }
    }

    pub fn is_button_dirty(&self, button_id: u8) -> bool {
        if (button_id as usize) < 10 {
            self.dirty_buttons[button_id as usize]
        } else {
            false
        }
    }

    pub fn is_strip_dirty(&self) -> bool {
        self.strip_dirty
    }

    pub fn has_any_dirty(&self) -> bool {
        self.strip_dirty || self.dirty_buttons.iter().any(|&d| d)
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_buttons = [false; 10];
        self.strip_dirty = false;
    }

    // --- Rendering ---

    /// Render a single button's 7x7 portion of the grid (112x112)
    pub fn render_button(&self, button_id: u8) -> RgbImage {
        let bid = button_id as usize;
        if bid >= 10 {
            return RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);
        }

        let mut img = RgbImage::new(BUTTON_WIDTH, BUTTON_HEIGHT);

        // Fill background
        for pixel in img.pixels_mut() {
            *pixel = BG_COLOR;
        }

        // Grid region for this button
        let grid_x0 = (bid % 5) * CELLS_PER_BUTTON;
        let grid_y0 = (bid / 5) * CELLS_PER_BUTTON;

        match self.state {
            GameState::Title => {
                self.render_title_button(&mut img, grid_x0, grid_y0);
            }
            GameState::Playing => {
                self.render_playing_button(&mut img, grid_x0, grid_y0);
            }
            GameState::GameOver => {
                self.render_gameover_button(&mut img, grid_x0, grid_y0);
            }
        }

        // Draw grid lines (1px gaps between cells)
        self.draw_grid_lines(&mut img);

        // Draw wall hints on edge buttons
        self.draw_wall_hints(&mut img, grid_x0, grid_y0);

        img
    }

    fn render_playing_button(&self, img: &mut RgbImage, grid_x0: usize, grid_y0: usize) {
        let snake_len = self.snake.len();

        // Draw snake segments
        for (i, seg) in self.snake.iter().enumerate() {
            let lx = seg.x as usize;
            let ly = seg.y as usize;
            if lx >= grid_x0
                && lx < grid_x0 + CELLS_PER_BUTTON
                && ly >= grid_y0
                && ly < grid_y0 + CELLS_PER_BUTTON
            {
                let color = if i == 0 {
                    HEAD_COLOR
                } else {
                    // Gradient from bright to dark along body
                    let t = if snake_len > 1 {
                        i as f32 / (snake_len - 1) as f32
                    } else {
                        0.0
                    };
                    lerp_color(BODY_BRIGHT, BODY_DARK, t)
                };
                self.draw_cell(img, lx - grid_x0, ly - grid_y0, color);
            }
        }

        // Draw food
        let fx = self.food.x as usize;
        let fy = self.food.y as usize;
        if fx >= grid_x0
            && fx < grid_x0 + CELLS_PER_BUTTON
            && fy >= grid_y0
            && fy < grid_y0 + CELLS_PER_BUTTON
        {
            let food_color = if self.food_pulse_on {
                FOOD_COLOR_B
            } else {
                FOOD_COLOR_A
            };
            self.draw_cell(img, fx - grid_x0, fy - grid_y0, food_color);
        }
    }

    fn render_title_button(&self, img: &mut RgbImage, grid_x0: usize, grid_y0: usize) {
        // "SNAKE" spelled out in block letters across the grid
        // Each letter is ~5 cells wide, ~7 cells tall, centered in 35x14 grid
        // Letters start at row 3 to center vertically
        let color = if self.blink_on {
            TITLE_COLOR_BRIGHT
        } else {
            TITLE_COLOR_DIM
        };

        // 5 letters, each 5 wide + 2 gap = 33 cols, offset by 1
        let letter_patterns = get_snake_letters();
        let letter_width = 5;
        let letter_gap = 2;
        let start_row = 4; // Center vertically

        for (li, pattern) in letter_patterns.iter().enumerate() {
            let letter_col = 1 + li * (letter_width + letter_gap);
            for (row, &bits) in pattern.iter().enumerate() {
                for col in 0..letter_width {
                    if bits & (1 << (letter_width - 1 - col)) != 0 {
                        let gx = letter_col + col;
                        let gy = start_row + row;
                        if gx >= grid_x0
                            && gx < grid_x0 + CELLS_PER_BUTTON
                            && gy >= grid_y0
                            && gy < grid_y0 + CELLS_PER_BUTTON
                        {
                            self.draw_cell(img, gx - grid_x0, gy - grid_y0, color);
                        }
                    }
                }
            }
        }
    }

    fn render_gameover_button(&self, img: &mut RgbImage, grid_x0: usize, grid_y0: usize) {
        if self.death_frame < 10 {
            // Death animation: red flash fading to dark
            let intensity = 1.0 - (self.death_frame as f32 / 10.0);

            // Draw snake body fading with red tint
            for seg in self.snake.iter() {
                let lx = seg.x as usize;
                let ly = seg.y as usize;
                if lx >= grid_x0
                    && lx < grid_x0 + CELLS_PER_BUTTON
                    && ly >= grid_y0
                    && ly < grid_y0 + CELLS_PER_BUTTON
                {
                    let r = (DEATH_FLASH[0] as f32 * intensity) as u8;
                    let g = (DEATH_FLASH[1] as f32 * intensity) as u8;
                    let b = (DEATH_FLASH[2] as f32 * intensity) as u8;
                    self.draw_cell(img, lx - grid_x0, ly - grid_y0, Rgb([r, g, b]));
                }
            }
        }
        // After death animation completes, buttons stay dark (just BG)
    }

    fn draw_cell(&self, img: &mut RgbImage, local_x: usize, local_y: usize, color: Rgb<u8>) {
        let px = local_x as u32 * CELL_PITCH;
        let py = local_y as u32 * CELL_PITCH;

        for dy in 0..CELL_SIZE {
            for dx in 0..CELL_SIZE {
                let x = px + dx;
                let y = py + dy;
                if x < BUTTON_WIDTH && y < BUTTON_HEIGHT {
                    img.put_pixel(x, y, color);
                }
            }
        }
    }

    fn draw_grid_lines(&self, img: &mut RgbImage) {
        // Draw subtle grid lines in the gaps between cells
        for i in 1..CELLS_PER_BUTTON {
            let pos = i as u32 * CELL_PITCH;
            // Vertical lines (in the 2px gap: at pos-2 and pos-1)
            if pos >= 1 {
                for y in 0..BUTTON_HEIGHT {
                    let gx = pos - 1;
                    if gx < BUTTON_WIDTH {
                        img.put_pixel(gx, y, GRID_COLOR);
                    }
                }
            }
            // Horizontal lines
            if pos >= 1 {
                for x in 0..BUTTON_WIDTH {
                    let gy = pos - 1;
                    if gy < BUTTON_HEIGHT {
                        img.put_pixel(x, gy, GRID_COLOR);
                    }
                }
            }
        }
    }

    fn draw_wall_hints(&self, img: &mut RgbImage, grid_x0: usize, grid_y0: usize) {
        // Left edge
        if grid_x0 == 0 {
            for y in 0..BUTTON_HEIGHT {
                img.put_pixel(0, y, WALL_HINT);
            }
        }
        // Right edge
        if grid_x0 + CELLS_PER_BUTTON >= GRID_COLS {
            let x = BUTTON_WIDTH - 1;
            for y in 0..BUTTON_HEIGHT {
                img.put_pixel(x, y, WALL_HINT);
            }
        }
        // Top edge
        if grid_y0 == 0 {
            for x in 0..BUTTON_WIDTH {
                img.put_pixel(x, 0, WALL_HINT);
            }
        }
        // Bottom edge
        if grid_y0 + CELLS_PER_BUTTON >= GRID_ROWS {
            let y = BUTTON_HEIGHT - 1;
            for x in 0..BUTTON_WIDTH {
                img.put_pixel(x, y, WALL_HINT);
            }
        }
    }

    /// Render the LCD strip HUD (800x128)
    pub fn render_strip(&self, font: &Font) -> RgbImage {
        let mut img = RgbImage::new(STRIP_WIDTH, STRIP_HEIGHT);

        // Fill background
        for pixel in img.pixels_mut() {
            *pixel = STRIP_BG;
        }

        match self.state {
            GameState::Title => {
                self.render_strip_title(&mut img, font);
            }
            GameState::Playing => {
                self.render_strip_playing(&mut img, font);
            }
            GameState::GameOver => {
                self.render_strip_gameover(&mut img, font);
            }
        }

        img
    }

    fn render_strip_title(&self, img: &mut RgbImage, font: &Font) {
        // "SNAKE" centered, with instructions
        let title = "SNAKE";
        let title_scale = 48.0;
        let tw = text_width(font, title, title_scale);
        let tx = ((STRIP_WIDTH as i32 - tw) / 2).max(0);
        let color = if self.blink_on {
            TITLE_COLOR_BRIGHT
        } else {
            TITLE_COLOR_DIM
        };
        draw_text(img, font, title, tx, 15, title_scale, color);

        // Instructions
        let instructions = "Press any button to start";
        let iw = text_width(font, instructions, 16.0);
        let ix = ((STRIP_WIDTH as i32 - iw) / 2).max(0);
        draw_text(img, font, instructions, ix, 80, 16.0, Rgb([80, 90, 110]));
    }

    fn render_strip_playing(&self, img: &mut RgbImage, font: &Font) {
        let padding = 20;

        // Left: Score
        draw_text(img, font, "SCORE", padding, 10, 14.0, Rgb([120, 130, 150]));
        let score_text = format!("{}", self.score);
        draw_text(img, font, &score_text, padding, 35, 36.0, HEAD_COLOR);

        // Center: Direction + speed bar
        let center_x = STRIP_WIDTH as i32 / 2;

        // Direction arrow and name (show pending direction for immediate feedback)
        let show_dir = self.effective_direction();
        let dir_text = format!("{} {}", show_dir.arrow(), show_dir.name());
        let dw = text_width(font, &dir_text, 24.0);
        let dx = center_x - dw / 2;
        let dir_color = if show_dir != self.direction {
            Rgb([50, 220, 100]) // Green highlight when turn is pending
        } else {
            Rgb([150, 160, 180])
        };
        draw_text(img, font, &dir_text, dx, 10, 24.0, dir_color);

        // Speed bar
        let bar_w: u32 = 200;
        let bar_h: u32 = 16;
        let bar_x = (center_x - bar_w as i32 / 2) as u32;
        let bar_y: u32 = 50;

        // Bar background
        draw_filled_rect(img, bar_x, bar_y, bar_w, bar_h, Rgb([30, 32, 42]));

        // Speed percentage: 250ms = 0%, 80ms = 100%
        let speed_pct = ((250.0 - self.move_interval_ms as f32) / (250.0 - 80.0)).clamp(0.0, 1.0);
        let fill_w = (bar_w as f32 * speed_pct) as u32;
        if fill_w > 0 {
            // Color gradient from green to yellow to red
            let color = if speed_pct < 0.5 {
                HEAD_COLOR
            } else if speed_pct < 0.8 {
                Rgb([200, 200, 50])
            } else {
                Rgb([255, 80, 50])
            };
            draw_filled_rect(img, bar_x, bar_y, fill_w, bar_h, color);
        }

        // Speed label
        let speed_label = "SPEED";
        let sw = text_width(font, speed_label, 11.0);
        let sx = center_x - sw / 2;
        draw_text(img, font, speed_label, sx, 72, 11.0, Rgb([80, 90, 110]));

        // Moves per second
        let mps = 1000.0 / self.move_interval_ms as f32;
        let mps_text = format!("{:.1}/s", mps);
        let mw = text_width(font, &mps_text, 14.0);
        let mx = center_x - mw / 2;
        draw_text(img, font, &mps_text, mx, 88, 14.0, Rgb([120, 130, 150]));

        // Right: High score
        let right_x = STRIP_WIDTH as i32 - padding;
        let hs_label = "HIGH";
        let hlw = text_width(font, hs_label, 14.0);
        draw_text(
            img,
            font,
            hs_label,
            right_x - hlw,
            10,
            14.0,
            Rgb([120, 130, 150]),
        );
        let hs_text = format!("{}", self.high_score);
        let hsw = text_width(font, &hs_text, 36.0);
        draw_text(
            img,
            font,
            &hs_text,
            right_x - hsw,
            35,
            36.0,
            Rgb([200, 180, 50]),
        );
    }

    fn render_strip_gameover(&self, img: &mut RgbImage, font: &Font) {
        // "GAME OVER" centered
        let title = "GAME OVER";
        let title_scale = 36.0;
        let tw = text_width(font, title, title_scale);
        let tx = ((STRIP_WIDTH as i32 - tw) / 2).max(0);

        let color = if self.death_frame < 10 {
            DEATH_FLASH
        } else if self.blink_on {
            Rgb([255, 80, 80])
        } else {
            Rgb([150, 40, 40])
        };
        draw_text(img, font, title, tx, 10, title_scale, color);

        // Score display
        let score_text = format!("Score: {}   High: {}", self.score, self.high_score);
        let sw = text_width(font, &score_text, 18.0);
        let sx = ((STRIP_WIDTH as i32 - sw) / 2).max(0);
        draw_text(img, font, &score_text, sx, 60, 18.0, Rgb([150, 160, 180]));

        // Instructions (only after death animation)
        if self.death_frame >= 10 {
            let restart = "Press button to restart";
            let rw = text_width(font, restart, 14.0);
            let rx = ((STRIP_WIDTH as i32 - rw) / 2).max(0);
            draw_text(img, font, restart, rx, 95, 14.0, Rgb([80, 90, 110]));
        }
    }
}

/// Get block letter patterns for "SNAKE" (5 wide x 7 tall each, as bitmasks)
fn get_snake_letters() -> [&'static [u8]; 5] {
    // S
    static S: [u8; 7] = [
        0b01110, // .###.
        0b10001, // #...#
        0b10000, // #....
        0b01110, // .###.
        0b00001, // ....#
        0b10001, // #...#
        0b01110, // .###.
    ];
    // N
    static N: [u8; 7] = [
        0b10001, // #...#
        0b11001, // ##..#
        0b10101, // #.#.#
        0b10101, // #.#.#
        0b10101, // #.#.#
        0b10011, // #..##
        0b10001, // #...#
    ];
    // A
    static A: [u8; 7] = [
        0b01110, // .###.
        0b10001, // #...#
        0b10001, // #...#
        0b11111, // #####
        0b10001, // #...#
        0b10001, // #...#
        0b10001, // #...#
    ];
    // K
    static K: [u8; 7] = [
        0b10001, // #...#
        0b10010, // #..#.
        0b10100, // #.#..
        0b11000, // ##...
        0b10100, // #.#..
        0b10010, // #..#.
        0b10001, // #...#
    ];
    // E
    static E: [u8; 7] = [
        0b11111, // #####
        0b10000, // #....
        0b10000, // #....
        0b11110, // ####.
        0b10000, // #....
        0b10000, // #....
        0b11111, // #####
    ];

    [&S, &N, &A, &K, &E]
}

/// Linearly interpolate between two colors
fn lerp_color(a: Rgb<u8>, b: Rgb<u8>, t: f32) -> Rgb<u8> {
    let t = t.clamp(0.0, 1.0);
    Rgb([
        (a[0] as f32 * (1.0 - t) + b[0] as f32 * t) as u8,
        (a[1] as f32 * (1.0 - t) + b[1] as f32 * t) as u8,
        (a[2] as f32 * (1.0 - t) + b[2] as f32 * t) as u8,
    ])
}
