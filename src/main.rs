use eframe::egui;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};
use rand::Rng;
use std::time::{Duration, Instant};

const HIGH_SCORE_FILE: &str = "high_scores.txt";
const GRID_WIDTH: usize = 40;
const GRID_HEIGHT: usize = 21;

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
    position: (i32, i32),
    shape: Vec<Vec<u8>>,
}

fn load_high_scores() -> Vec<(String, i32)> {
    if let Ok(file) = fs::File::open(HIGH_SCORE_FILE) {
        io::BufReader::new(file)
            .lines()
            .filter_map(|line| {
                let line = line.ok()?;
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() == 2 {
                    let name = parts[0].to_string();
                    if let Ok(score) = parts[1].parse::<i32>() {
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
            drop_speed: Duration::from_millis(125),
            selected_difficulty: None,
        }
    }
}

impl CrowsTetris {
    fn reset_game(&mut self) {
        self.state = GameState::Playing;
        self.score = 0;
        self.is_paused = false;
        self.grid = [[0; GRID_WIDTH]; GRID_HEIGHT];
        self.active_block = Some(self.generate_random_block());
    }

    fn generate_random_block(&self) -> Block {
        let block_type = match rand::rng().random_range(0..7) {
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
            position: (GRID_WIDTH as i32 / 2 - shape[0].len() as i32 / 2, 0), // Starts at the top center
            shape,
        }

    }

    fn move_block_down(&mut self) {
        if let Some(block) = self.active_block.as_ref() {
            let position = block.position;
            let collided = self.check_collision_with_position(position);

            if !collided {
                let mut blck = self.active_block.as_mut().unwrap();
                blck.position.1 += 1;
            } else {
                self.lock_block();  
                self.clear_lines();
                self.active_block = Some(self.generate_random_block());

                let new_block = self.generate_random_block();
                if self.check_collision_with_position(new_block.position) {
                    self.state = GameState::GameOver;
                } else {
                    self.active_block = Some(new_block);
                }
            }
        }
    }

    fn check_collision_with_position(&self, position: (i32, i32)) -> bool {
        let (x, y) = position;

        if let Some(block) = &self.active_block {
            for (dy, row) in block.shape.iter().enumerate() {
                for (dx, cell) in row.iter().enumerate() {
                    if *cell != 0 {
                        let grid_x = x + dx as i32;
                        let grid_y = y + dy as i32;

                        if grid_x < 0 || grid_x >= (GRID_WIDTH as i32) - 1 || grid_y >= (GRID_HEIGHT as i32) - 1 {
                            return true;
                        }

                        if self.grid[grid_y as usize][grid_x as usize] != 0 {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn lock_block(&mut self) {
        if let Some(block) = &self.active_block {
            for (dy, row) in block.shape.iter().enumerate() {
                for (dx, &cell) in row.iter().enumerate() {
                    if cell == 1 {
                        let x = block.position.0 + dx as i32;
                        let y = block.position.1 + dy as i32;
                        if y >= 0 && x >= 0 && x < GRID_WIDTH as i32 && y < GRID_HEIGHT as i32 {
                            self.grid[y as usize][x as usize] = cell;
                        }
                    }
                }
            }
        }
    }

    fn clear_lines(&mut self) {
        let mut new_grid = [[0; GRID_WIDTH]; GRID_HEIGHT];
        let mut new_row = GRID_HEIGHT - 1;

        for y in (0..GRID_HEIGHT).rev() {
            // Copy non-full rows downward
            if !self.grid[y].iter().all(|&cell| cell == 1) {
                new_grid[new_row] = self.grid[y];
                if new_row > 0 {
                    new_row -= 1;
                }
            } else {
                self.score += 100;
            }
        }

        self.grid = new_grid;
    }

    fn render_grid(&self, ui: &mut egui::Ui) {
        let mut grid_with_block = self.grid.clone();

        if let Some(block) = &self.active_block {
            for (dy, row) in block.shape.iter().enumerate() {
                for (dx, &cell) in row.iter().enumerate() {
                    if cell == 1 {
                        let x = block.position.0 + dx as i32;
                        let y = block.position.1 + dy as i32;
                        if x >= 0 && x < GRID_WIDTH as i32 && y >= 0 && y < GRID_HEIGHT as i32 {
                            grid_with_block[y as usize][x as usize] = 1;
                        }
                    }
                }
            }
        }
        for row in &grid_with_block {
            let row_str: String = row.iter().map(|&cell| if cell == 1 { "■" } else { "0" }).collect();
            //println!("{}", row_str);
            ui.label(row_str);
        }

        if let Some(block) = &self.active_block {
            ui.label(format!("Active Block at {:?}", block.position));
        }
    }

    fn rotate_block(&mut self) {
        if let Some(block) = self.active_block.as_ref() {
            let original_shape = block.shape.clone();
            let rotated_shape: Vec<Vec<u8>> = (0..block.shape[0].len())
                .map(|i| block.shape.iter().rev().map(|row| row[i]).collect())
                .collect();


            if !self.check_collision_with_position(block.position) {
                let mut blck = self.active_block.as_mut().unwrap();
                blck.shape = rotated_shape;
            }
        }
    }
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
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::DARK_RED))
            .show(ctx, |ui| {
                let score_label = egui::RichText::new(format!("Score: {}", self.score))
                    .size(21.0)
                    .strong();

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    ui.label(score_label);
                    ui.add_space(20.0);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {

                    ui.add_space(20.0);
                    ui.label("Level: 0");
                });



                ui.add_space(10.0);

                if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                    self.is_paused = !self.is_paused;
                }

                let now = Instant::now();
                if now.duration_since(self.last_update) >= self.drop_speed {
                    self.last_update = now; // Reset timer at actual execution
                    self.move_block_down();
                }

                if self.is_paused {
                    ui.vertical_centered(|ui| {
                        ui.label("Game Paused");
                    });

                    return;
                }

                self.render_grid(ui);

                if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                    let new_position = self.active_block.as_ref()
                        .map(|block| (block.position.0 - 1, block.position.1))
                        .unwrap_or((0, 0));

                    let has_collision = self.check_collision_with_position(new_position);

                    if !has_collision {
                        if let Some(block) = self.active_block.as_mut() {
                            block.position.0 -= 1;
                        }
                    }
                }

                if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                    let new_position = self.active_block.as_ref()
                        .map(|block| (block.position.0 + 1, block.position.1))
                        .unwrap_or((0, 0));

                    let has_collision = self.check_collision_with_position(new_position);

                    if !has_collision {
                        if let Some(block) = self.active_block.as_mut() {
                            block.position.0 += 1;
                        }
                    }
                }

                if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    self.rotate_block();
                    ui.label("Rotated");
                }
                if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    ui.label("Moved Down");
                }

                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.state = GameState::GameOver;
                }
            });
    }

    fn render_game_over(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Game Over!");
                ui.add_space(140.0);
                ui.label("Enter Name:");
                ui.text_edit_singleline(&mut self.new_high_score_name);

                ui.add_space(33.0);
                if ui.button("Submit Score").clicked() && !self.new_high_score_name.is_empty() {
                    self.high_scores.push((self.new_high_score_name.clone(), self.score));
                    self.high_scores
                        .sort_by(|a, b| b.1.cmp(&a.1));
                    self.high_scores.truncate(10);
                    save_high_scores(&self.high_scores);
                    self.new_high_score_name.clear();
                    self.state = GameState::StartScreen;
                }

                ui.add_space(33.0);
                if ui.button("Back to Start").clicked() {
                    self.state = GameState::StartScreen;
                }
            });
        });
    }
}

fn main() {
    let app = CrowsTetris::default();
    let ctx = egui::Context::default();
    let mut size = ctx.used_size();
    size.x = 420.00;
    size.y = 540.00;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_resizable(false)
            .with_inner_size(size),
        ..Default::default()
    };

    let _ = eframe::run_native("Crow's Tetris", options, Box::new( |_cc| Ok(Box::new(app)) ));
}