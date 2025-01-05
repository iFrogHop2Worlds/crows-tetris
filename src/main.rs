use eframe::egui;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};
use rand::Rng;
use std::time::{Duration, Instant};

const HIGH_SCORE_FILE: &str = "high_scores.txt";
const GRID_WIDTH: usize = 10;
const GRID_HEIGHT: usize = 20;

struct CrowsTetris {
    state: GameState,
    score: i32,
    high_scores: Vec<(String, i32)>,
    new_high_score_name: String,
    is_paused: bool,
    grid: [[u8; GRID_WIDTH]; GRID_HEIGHT],
    active_block: Option<Block>,
    last_update: Instant, // Timer for block movement
    drop_speed: Duration,
    selected_difficulty: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum GameState {
    StartScreen,
    Playing,
    GameOver,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum BlockType {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

#[derive(Debug, Clone)]
struct Block {
    block_type: BlockType,
    position: (i32, i32),      // (x, y): Coordinates on the grid
    shape: Vec<Vec<u8>>,      // 2D shape of the block (1 for filled, 0 for empty)
}

fn load_high_scores() -> Vec<(String, i32)> {
    if let Ok(file) = fs::File::open(HIGH_SCORE_FILE) {
        io::BufReader::new(file)
            .lines()
            .filter_map(|line| {
                let line = line.ok()?; // Read the line
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() == 2 {
                    let name = parts[0].to_string(); // Convert to String
                    if let Ok(score) = parts[1].parse::<i32>() { // Parse score
                        return Some((name, score));
                    }
                }
                None
            })
            .collect()
    } else {
        vec![]
    }
}

fn save_high_scores(high_scores: &[(String, i32)]) {
    if let Ok(mut file) = OpenOptions::new()
        .write(true)
        .create(true)
        .open(HIGH_SCORE_FILE)
    {
        for (name, score) in high_scores {
            writeln!(file, "{},{}", name, score).ok();
        }
    }
}

impl Default for CrowsTetris {
    fn default() -> Self {
        Self {
            state: GameState::StartScreen,
            score: 0,
            high_scores: load_high_scores(),
            new_high_score_name: String::new(),
            is_paused: false,
            grid: [[0; GRID_WIDTH]; GRID_HEIGHT],
            active_block: None,
            last_update: Instant::now(),
            drop_speed: Duration::from_millis(500), // Default speed (medium)
            selected_difficulty: None,
        }
    }
}

impl CrowsTetris {
    /// Resets the game and initializes a new grid and starting block
    fn reset_game(&mut self) {
        self.state = GameState::Playing;
        self.score = 0;
        self.is_paused = false;
        self.grid = [[0; GRID_WIDTH]; GRID_HEIGHT];
        self.active_block = Some(self.generate_random_block());
    }

    /// Generates a random Tetris block with a shape
    fn generate_random_block(&self) -> Block {
        let block_type = match rand::thread_rng().gen_range(0..7) {
            0 => BlockType::I,
            1 => BlockType::O,
            2 => BlockType::T,
            3 => BlockType::S,
            4 => BlockType::Z,
            5 => BlockType::J,
            _ => BlockType::L,
        };
        let shape = match block_type {
            BlockType::I => vec![vec![1, 1, 1, 1]],
            BlockType::O => vec![vec![1, 1], vec![1, 1]],
            BlockType::T => vec![vec![0, 1, 0], vec![1, 1, 1]],
            BlockType::S => vec![vec![0, 1, 1], vec![1, 1, 0]],
            BlockType::Z => vec![vec![1, 1, 0], vec![0, 1, 1]],
            BlockType::J => vec![vec![1, 0, 0], vec![1, 1, 1]],
            BlockType::L => vec![vec![0, 0, 1], vec![1, 1, 1]],
        };
        Block {
            block_type,
            position: (GRID_WIDTH as i32 / 2 - 1, 0), // Starts at the top center
            shape,
        }
    }

    fn move_block_down(&mut self) {
        // Step 1: Manage mutable borrow
        let mut collided = false; // Flag for collision detection
        if let Some(block) = self.active_block.as_mut() {
            block.position.1 += 1; // Move the block down
            collided = self.check_collision(); // Flag collision after move
        }

        // Step 2: After mutable borrow ends, handle collision logic
        if collided {
            if let Some(block) = self.active_block.as_mut() {
                block.position.1 -= 1; // Revert the move
            }
            self.lock_block(); // Lock block in place
            self.clear_lines(); // Clear any completed lines
            self.active_block = Some(self.generate_random_block());

            // Check for a game over condition after spawning a new block
            if self.check_collision() {
                self.state = GameState::GameOver;
            }
        }
    }

    /// Checks if the active block collides with the grid or boundaries
    fn check_collision(&self) -> bool {
        if let Some(block) = &self.active_block {
            for (dy, row) in block.shape.iter().enumerate() {
                for (dx, &cell) in row.iter().enumerate() {
                    if cell == 1 {
                        let x = block.position.0 + dx as i32;
                        let y = block.position.1 + dy as i32;
                        if x < 0 || x >= GRID_WIDTH as i32 || y >= GRID_HEIGHT as i32 {
                            return true; // Outside boundaries
                        }
                        if y >= 0 && self.grid[y as usize][x as usize] == 1 {
                            return true; // Collides with existing block
                        }
                    }
                }
            }
        }
        false
    }

    /// Locks the active block into the grid
    fn lock_block(&mut self) {
        if let Some(block) = &self.active_block {
            for (dy, row) in block.shape.iter().enumerate() {
                for (dx, &cell) in row.iter().enumerate() {
                    if cell == 1 {
                        let x = block.position.0 + dx as i32;
                        let y = block.position.1 + dy as i32;
                        if y >= 0 {
                            self.grid[y as usize][x as usize] = 1;
                        }
                    }
                }
            }
        }
    }

    /// Clears completed lines and updates the score
    fn clear_lines(&mut self) {
        let mut new_grid = [[0; GRID_WIDTH]; GRID_HEIGHT];
        let mut new_row = GRID_HEIGHT - 1;
        for y in (0..GRID_HEIGHT).rev() {
            if self.grid[y].iter().all(|&cell| cell == 1) {
                self.score += 100; // Increase score for each cleared line
                continue;
            }
            new_grid[new_row] = self.grid[y];
            // Gaurds underflow
            if 1 > new_row {
                return
            } else {
                new_row -= 1;
            }

        }
        self.grid = new_grid;
    }

    /// Renders the grid and active block
    fn render_grid(&self, ui: &mut egui::Ui) {
        for row in &self.grid {
            let row_str: String = row.iter().map(|&cell| if cell == 1 { "■" } else { "." }).collect();
            ui.label(row_str);
        }
        if let Some(block) = &self.active_block {
            ui.label(format!("Active Block at {:?}", block.position));
        }
    }

    // Handles gameplay rendering and block movement
    // fn render_gameplay(&mut self, ctx: &egui::Context) {
    //     egui::CentralPanel::default().show(ctx, |ui| {
    //         ui.label(format!("Score: {}", self.score));
    //         if self.is_paused {
    //             ui.label("Game Paused");
    //             return;
    //         }
    //         if self.last_update.elapsed() >= self.drop_speed {
    //             self.last_update = Instant::now();
    //             self.move_block_down();
    //         }
    //         self.render_grid(ui);
    //     });
    // }
}

impl eframe::App for CrowsTetris {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        match self.state {
            GameState::StartScreen => self.render_start_screen(ctx),
            GameState::Playing => self.render_gameplay(ctx),
            GameState::GameOver => self.render_game_over(ctx),
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Q)) {
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            });
        }
    }
}

impl CrowsTetris {
    fn render_start_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Crow's Tetris");
                ui.add_space(10.0);

                if ui.button("Start Game").clicked() {
                    self.reset_game();
                }

                ui.add_space(30.0);
                ui.heading("High Scores:");
                for (i, (name, score)) in self.high_scores.iter().take(10).enumerate() {
                    ui.label(format!("{}. {} - {}", i + 1, name, score));
                }
            });
        });
    }

    fn render_gameplay(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Display score at the top
            let score_label = egui::RichText::new(format!("Score: {}", self.score))
                .size(21.0)
                .strong();
            ui.vertical_centered(|ui| ui.label(score_label));

            ui.add_space(10.0);

            // Pause functionality
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                self.is_paused = !self.is_paused;
            }

            if self.last_update.elapsed() >= self.drop_speed {
                self.last_update = Instant::now();
                self.move_block_down();
            }

            if self.is_paused {
                ui.vertical_centered(|ui| {
                    ui.label("Game Paused");
                });

                return;
            }

            // Example gameplay mechanics (placeholder for actual Tetris logic)
            ui.label("Gameplay goes here...");

            // Handle movement (left, right, rotate)
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                ui.label("Moved Left");
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                ui.label("Moved Right");
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                ui.label("Rotated");
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                ui.label("Moved Down");
            }

            // If the game is over, transition to GameOver state
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.state = GameState::GameOver;
            }
        });
    }

    fn render_game_over(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Game Over!");

                // Input for entering a high score name if applicable
                ui.horizontal(|ui| {
                    ui.label("Enter Name:");
                    ui.text_edit_singleline(&mut self.new_high_score_name);
                });

                if ui.button("Submit Score").clicked() && !self.new_high_score_name.is_empty() {
                    self.high_scores.push((self.new_high_score_name.clone(), self.score));
                    self.high_scores
                        .sort_by(|a, b| b.1.cmp(&a.1));
                    self.high_scores.truncate(10);
                    save_high_scores(&self.high_scores);
                    self.new_high_score_name.clear();
                    self.state = GameState::StartScreen;
                }

                if ui.button("Back to Start").clicked() {
                    self.state = GameState::StartScreen;
                }
            });
        });
    }
}

fn main() {
    let app = CrowsTetris::default();
    let options = eframe::NativeOptions::default();
    let _ = eframe::run_native("Crow's Tetris", options, Box::new( |_cc| Ok(Box::new(app)) ));
}