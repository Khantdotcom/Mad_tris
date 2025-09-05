use std::fs;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute, queue, style, terminal,
};
use rand::{prelude::ThreadRng, Rng};
use serde::{Deserialize, Serialize};

// --- CONFIGURATION & COMMAND-LINE ARGS ---

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of columns on the board
    #[arg(long, default_value_t = 10)]
    columns: usize,
    /// Number of lines on the board
    #[arg(long, default_value_t = 20)]
    lines: usize,
}

// --- COLOR & PIECE DEFINITIONS ---

// Added Serialize and Deserialize for saving/loading the game state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Color(u8, u8, u8);

struct Piece {
    rotations: &'static [(usize, &'static [u8])],
    color: Color,
}

// Static definitions for the 7 classic Tetris pieces (tetrominos).
static PIECES: [Piece; 7] = [
    // I-Block
    Piece { rotations: &[(4, &[1,1,1,1]), (1, &[1,1,1,1])], color: Color(3, 252, 248) },
    // O-Block
    Piece { rotations: &[(2, &[1,1,1,1])], color: Color(252, 244, 3) },
    // T-Block
    Piece { rotations: &[(3, &[0,1,0,1,1,1]), (2, &[1,0,1,1,1,0]), (3, &[1,1,1,0,1,0]), (2, &[0,1,1,1,0,1])], color: Color(161, 3, 252) },
    // L-Block
    Piece { rotations: &[(3, &[0,0,1,1,1,1]), (2, &[1,0,1,0,1,1]), (3, &[1,1,1,1,0,0]), (2, &[1,1,0,1,0,1])], color: Color(252, 161, 3) },
    // J-Block
    Piece { rotations: &[(3, &[1,0,0,1,1,1]), (2, &[1,1,1,0,1,0]), (3, &[1,1,1,0,0,1]), (2, &[0,1,0,1,1,1])], color: Color(3, 48, 252) },
    // S-Block
    Piece { rotations: &[(3, &[0,1,1,1,1,0]), (2, &[1,0,1,1,0,1])], color: Color(3, 252, 28) },
    // Z-Block
    Piece { rotations: &[(3, &[1,1,0,0,1,1]), (2, &[0,1,1,1,1,0])], color: Color(252, 3, 3) },
];

// --- ACTIVE PIECE ---

// Added Serialize and Deserialize for saving/loading the game state.
#[derive(Clone, Serialize, Deserialize)]
struct ActivePiece {
    id: usize,
    rotation: usize,
    x: isize,
    y: isize,
}

impl ActivePiece {
    fn new(id: usize, board_width: usize) -> Self {
        let width = PIECES[id].rotations[0].0;
        ActivePiece {
            id,
            rotation: 0,
            x: (board_width as isize - width as isize) / 2,
            y: 0,
        }
    }

    fn definition(&self) -> &Piece { &PIECES[self.id] }
    fn width(&self) -> usize { self.definition().rotations[self.rotation].0 }
    fn bitmap(&self) -> &'static [u8] { self.definition().rotations[self.rotation].1 }

    fn blocks(&self) -> impl Iterator<Item = (isize, isize)> + '_ {
        let width = self.width();
        self.bitmap()
            .iter()
            .enumerate()
            .filter(|&(_, &cell)| cell == 1)
            .map(move |(i, _)| (self.x + (i % width) as isize, self.y + (i / width) as isize))
    }
}

// --- SAVEGAME STATE ---
// A separate struct for serialization that holds all data needed to restore a game.
#[derive(Serialize, Deserialize)]
struct SerializableGameState {
    board: Vec<Option<Color>>,
    width: usize,
    height: usize,
    active_piece: ActivePiece,
    next_piece_id: usize,
    is_game_over: bool,
    gravity_delay_ms: u64,
    speed_up_counter: usize,
    score: u32,
}

// --- GAME STATE & LOGIC ---

struct Game {
    board: Vec<Option<Color>>,
    width: usize,
    height: usize,
    active_piece: ActivePiece,
    next_piece_id: usize,
    rng: ThreadRng,
    is_game_over: bool,
    paused: bool,
    gravity_delay: Duration,
    last_gravity_time: Instant,
    speed_up_counter: usize,
    score: u32,
    status_message: Option<(String, Instant)>,
}

impl Game {
    fn new(width: usize, height: usize) -> Self {
        let mut rng = rand::thread_rng();
        let first_piece_id = rng.gen_range(0..PIECES.len());
        let next_piece_id = rng.gen_range(0..PIECES.len());
        Game {
            board: vec![None; width * height],
            width,
            height,
            active_piece: ActivePiece::new(first_piece_id, width),
            rng,
            is_game_over: false,
            paused: false,
            gravity_delay: Duration::from_millis(1000),
            last_gravity_time: Instant::now(),
            speed_up_counter: 0,
            score: 0,
            next_piece_id,
            status_message: None,
        }
    }

    fn check_collision(&self, piece: &ActivePiece) -> bool {
        piece.blocks().any(|(x, y)| {
            x < 0
                || x >= self.width as isize
                || y >= self.height as isize
                || (y >= 0 && self.board[(y as usize * self.width) + x as usize].is_some())
        })
    }
    
    fn spawn_new_piece(&mut self) {
        self.speed_up_counter += 1;
        if self.speed_up_counter >= 10 {
            let new_millis = self.gravity_delay.as_millis().saturating_sub(75).max(150) as u64;
            self.gravity_delay = Duration::from_millis(new_millis);
            self.speed_up_counter = 0;
        }

        self.active_piece = ActivePiece::new(self.next_piece_id, self.width);
        self.next_piece_id = self.rng.gen_range(0..PIECES.len());

        if self.check_collision(&self.active_piece) {
            self.is_game_over = true;
        }
    }

    fn try_move(&mut self, dx: isize, dy: isize) -> bool {
        let mut test_piece = self.active_piece.clone();
        test_piece.x += dx;
        test_piece.y += dy;
        if !self.check_collision(&test_piece) {
            self.active_piece = test_piece;
            return true;
        }
        false
    }

    fn try_rotate(&mut self) {
        let mut test_piece = self.active_piece.clone();
        let num_rotations = test_piece.definition().rotations.len();
        test_piece.rotation = (test_piece.rotation + 1) % num_rotations;

        // Wall kick attempts
        for offset in [0, 1, -1, 2, -2] {
            let original_x = self.active_piece.x;
            test_piece.x = original_x + offset;
            if !self.check_collision(&test_piece) {
                self.active_piece = test_piece;
                return;
            }
        }
    }

    fn lock_piece(&mut self) {
        let color = self.active_piece.definition().color;
        for (x, y) in self.active_piece.blocks() {
            if y >= 0 {
                self.board[(y as usize * self.width) + x as usize] = Some(color);
            }
        }
        self.clear_lines();
        self.spawn_new_piece();
    }

    fn clear_lines(&mut self) {
        let mut new_board = vec![None; self.width * self.height];
        let mut cleared_lines_count = 0;
        let mut new_row_index = self.height - 1;

        for y in (0..self.height).rev() {
            let row_start = y * self.width;
            let row = &self.board[row_start..row_start + self.width];

            if row.iter().all(|cell| cell.is_some()) {
                cleared_lines_count += 1;
            } else {
                if new_row_index < self.height {
                    let new_row_start = new_row_index * self.width;
                    new_board[new_row_start..new_row_start + self.width].copy_from_slice(row);
                }
                new_row_index = new_row_index.saturating_sub(1);
            }
        }
        self.board = new_board;

        let points = match cleared_lines_count {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        };
        self.score += points;
    }
    
    fn update(&mut self) {
        // Clear status message after a couple of seconds
        if let Some((_, time)) = self.status_message {
            if time.elapsed() > Duration::from_secs(2) {
                self.status_message = None;
            }
        }
        if self.is_game_over || self.paused {
            return;
        }
        if self.last_gravity_time.elapsed() >= self.gravity_delay {
            if !self.try_move(0, 1) {
                self.lock_piece();
            }
            self.last_gravity_time = Instant::now();
        }
    }

    fn render<W: Write>(&self, w: &mut W) -> io::Result<()> {
        queue!(w, cursor::Hide, terminal::Clear(terminal::ClearType::All))?;

        let board_top_y: u16 = 1;
        let board_left_x: u16 = 1;

        queue!(w, cursor::MoveTo(board_left_x, board_top_y - 1), style::Print(format!("╔{}╗", "═".repeat(self.width * 2))))?;
        for y in 0..self.height {
            queue!(w, cursor::MoveTo(board_left_x, board_top_y + y as u16), style::Print("║"))?;
            for x in 0..self.width {
                let bg_char = if (x + y) % 2 == 0 { "·" } else { " " };
                queue!(w, style::SetForegroundColor(style::Color::DarkGrey), style::Print(bg_char.repeat(2)))?;
            }
            queue!(w, style::SetForegroundColor(style::Color::White), style::Print("║"))?;
        }
        queue!(w, cursor::MoveTo(board_left_x, board_top_y + self.height as u16), style::Print(format!("╚{}╝","═".repeat(self.width * 2))))?;

        let draw_block = |w: &mut W, x: isize, y: isize, color: Color| -> io::Result<()> {
            let Color(r, g, b) = color;
            queue!(w, cursor::MoveTo((board_left_x as isize +1+ x * 2) as u16, (board_top_y as isize + y) as u16),
                style::SetForegroundColor(style::Color::Rgb { r, g, b }),
                style::Print("██"))?;
            Ok(())
        };

        for (i, cell) in self.board.iter().enumerate() {
            if let Some(color) = cell {
                draw_block(w, (i % self.width) as isize, (i / self.width) as isize, *color)?;
            }
        }

        if !self.is_game_over {
            let color = self.active_piece.definition().color;
            for (x, y) in self.active_piece.blocks() {
                if y >= 0 {
                    draw_block(w, x, y, color)?;
                }
            }
        }

        let panel_x = (self.width * 2 + 5) as u16;
        queue!(w, cursor::MoveTo(panel_x, 2), style::SetForegroundColor(style::Color::White), style::Print("Score"))?;
        queue!(w, cursor::MoveTo(panel_x, 3), style::SetForegroundColor(style::Color::Yellow), style::Print(format!("{:0>8}", self.score)))?;

        queue!(w, cursor::MoveTo(panel_x, 5), style::SetForegroundColor(style::Color::White), style::Print("Next Piece"))?;
        let next_piece = &PIECES[self.next_piece_id];
        let (p_width, p_bitmap) = next_piece.rotations[0];
        for (i, &cell) in p_bitmap.iter().enumerate() {
            if cell == 1 {
                let x = (i % p_width) as isize;
                let y = (i / p_width) as isize;
                let Color(r,g,b) = next_piece.color;
                queue!(w, cursor::MoveTo(panel_x + (x * 2) as u16, 6 + y as u16),
                    style::SetForegroundColor(style::Color::Rgb { r, g, b }),
                    style::Print("██"))?;
            }
        }

        let controls_y = 12;
        queue!(w, cursor::MoveTo(panel_x, controls_y), style::SetForegroundColor(style::Color::White), style::Print("Controls"))?;
        queue!(w, cursor::MoveTo(panel_x, controls_y + 1), style::Print("←/→: Move"))?;
        queue!(w, cursor::MoveTo(panel_x, controls_y + 2), style::Print("  ↑: Rotate"))?;
        queue!(w, cursor::MoveTo(panel_x, controls_y + 3), style::Print("  ↓: Soft Drop"))?;
        queue!(w, cursor::MoveTo(panel_x, controls_y + 4), style::Print("Spc: Hard Drop"))?;
        queue!(w, cursor::MoveTo(panel_x, controls_y + 5), style::Print("  P: Pause"))?;
        queue!(w, cursor::MoveTo(panel_x, controls_y + 6), style::Print("  S: Save"))?;
        queue!(w, cursor::MoveTo(panel_x, controls_y + 7), style::Print("  L: Load"))?;
        queue!(w, cursor::MoveTo(panel_x, controls_y + 8), style::Print("  Q: Quit"))?;
        
        if self.is_game_over {
            let msg = "GAME OVER";
            let msg_x = board_left_x + ((self.width * 2 - msg.len()) / 2) as u16;
            let msg_y = board_top_y + (self.height / 2) as u16;
            queue!(w, cursor::MoveTo(msg_x, msg_y), style::SetForegroundColor(style::Color::Red), style::Print(msg))?;
        } else if self.paused {
            let msg = "PAUSED";
            let msg_x = board_left_x + ((self.width * 2 - msg.len()) / 2) as u16;
            let msg_y = board_top_y + (self.height / 2) as u16;
            queue!(w, cursor::MoveTo(msg_x, msg_y), style::SetForegroundColor(style::Color::Cyan), style::Print(msg))?;
        }

        if let Some((msg, _)) = &self.status_message {
            let msg_x = board_left_x + ((self.width * 2 - msg.len()) / 2) as u16;
            let msg_y = board_top_y + self.height as u16 + 1;
            queue!(w, cursor::MoveTo(msg_x, msg_y), style::SetForegroundColor(style::Color::Green), style::Print(msg))?;
        }

        w.flush()
    }

    fn run<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        'running: loop {
            while event::poll(Duration::from_millis(1))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break 'running,
                        _ => {}
                    }
                    if self.is_game_over && key.code != KeyCode::Char('l') && key.code != KeyCode::Char('L') { continue; }

                    if !self.paused {
                         match key.code {
                            KeyCode::Left if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                                self.try_move(-1, 0);
                            }
                            KeyCode::Right if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                                self.try_move(1, 0);
                            }
                            KeyCode::Up if key.kind == KeyEventKind::Press => {
                                self.try_rotate();
                            }
                            KeyCode::Down if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                                if self.try_move(0, 1) {
                                    self.last_gravity_time = Instant::now();
                                } else {
                                    self.lock_piece();
                                    self.last_gravity_time = Instant::now();
                                }
                            }
                            KeyCode::Char(' ') if key.kind == KeyEventKind::Press => {
                                let mut distance = 0;
                                while self.try_move(0, 1) { distance += 1; }
                                self.lock_piece();
                                self.last_gravity_time = Instant::now();
                            }
                            _ => {}
                        }
                    }

                    match key.code {
                         KeyCode::Char('p') | KeyCode::Char('P') if key.kind == KeyEventKind::Press => {
                            self.paused = !self.paused;
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') if key.kind == KeyEventKind::Press => {
                            match self.save_game() {
                                Ok(_) => self.set_status_message("Game Saved!".to_string()),
                                Err(e) => self.set_status_message(format!("Save Failed: {}", e)),
                            }
                        }
                        KeyCode::Char('l') | KeyCode::Char('L') if key.kind == KeyEventKind::Press => {
                            match self.load_game() {
                                Ok(_) => self.set_status_message("Game Loaded!".to_string()),
                                Err(e) => self.set_status_message(format!("Load Failed: {}", e)),
                            }
                        }
                        _ => {}
                    }
                }
            }

            self.update();
            self.render(writer)?;
            std::thread::sleep(Duration::from_millis(16));
        }
        Ok(())
    }

    fn set_status_message(&mut self, msg: String) {
        self.status_message = Some((msg, Instant::now()));
    }

    fn save_game(&self) -> io::Result<()> {
        let state = SerializableGameState {
            board: self.board.clone(),
            width: self.width,
            height: self.height,
            active_piece: self.active_piece.clone(),
            next_piece_id: self.next_piece_id,
            is_game_over: self.is_game_over,
            gravity_delay_ms: self.gravity_delay.as_millis() as u64,
            speed_up_counter: self.speed_up_counter,
            score: self.score,
        };
        let serialized = serde_json::to_string(&state)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write("tetris_save.json", serialized)
    }

    fn load_game(&mut self) -> io::Result<()> {
        let data = fs::read_to_string("tetris_save.json")?;
        let state: SerializableGameState = serde_json::from_str(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        
        self.board = state.board;
        self.width = state.width;
        self.height = state.height;
        self.active_piece = state.active_piece;
        self.next_piece_id = state.next_piece_id;
        self.is_game_over = state.is_game_over;
        self.gravity_delay = Duration::from_millis(state.gravity_delay_ms);
        self.speed_up_counter = state.speed_up_counter;
        self.score = state.score;
        self.paused = false; // Always unpause on load
        self.last_gravity_time = Instant::now(); // Reset gravity timer

        Ok(())
    }
}

// --- NEW HELPER FUNCTIONS ---

/// Loads the high score from "highscore.txt". Returns 0 if the file doesn't exist or contains invalid data.
fn load_high_score() -> u32 {
    fs::read_to_string("highscore.txt")
        .unwrap_or_else(|_| "0".to_string())
        .trim()
        .parse()
        .unwrap_or(0)
}

/// Saves the given score to "highscore.txt", overwriting it.
fn save_high_score(score: u32) -> io::Result<()> {
    fs::write("highscore.txt", score.to_string())
}

/// Consumes and discards all pending input events from the queue.
fn drain_event_queue() -> io::Result<()> {
    while event::poll(Duration::from_millis(0))? {
        let _ = event::read()?;
    }
    Ok(())
}

/// Displays a centered start screen and waits for any key press.
fn show_start_screen<W: Write>(w: &mut W) -> io::Result<()> {
    let (width, height) = terminal::size()?;
    let title = "RUST TETRIS";
    let msg = "Press any key to start";

    queue!(w, terminal::Clear(terminal::ClearType::All))?;
    queue!(w, cursor::MoveTo((width - title.len() as u16) / 2, height / 2 - 2))?;
    queue!(w, style::SetForegroundColor(style::Color::Yellow), style::Print(title))?;

    queue!(w, cursor::MoveTo((width - msg.len() as u16) / 2, height / 2))?;
    queue!(w, style::SetForegroundColor(style::Color::White), style::Print(msg))?;
    w.flush()?;

    // Block until any key is pressed
    event::read()?;
    Ok(())
}

/// Displays the end screen with final score, high score, and options.
fn show_end_screen<W: Write>(w: &mut W, score: u32, high_score: u32) -> io::Result<()> {
    let (width, height) = terminal::size()?;
    let title = "GAME OVER";
    let score_text = format!("Final Score: {}", score);
    let high_score_text = format!("High Score: {}", high_score);
    let msg = "R: Restart, Q: Quit";

    queue!(w, terminal::Clear(terminal::ClearType::All))?;

    queue!(w, cursor::MoveTo((width - title.len() as u16) / 2, height / 2 - 3))?;
    queue!(w, style::SetForegroundColor(style::Color::Red), style::Print(title))?;

    queue!(w, cursor::MoveTo((width - score_text.len() as u16) / 2, height / 2 - 1))?;
    queue!(w, style::SetForegroundColor(style::Color::White), style::Print(score_text))?;


    queue!(w, cursor::MoveTo((width - high_score_text.len() as u16) / 2, height / 2))?;
    queue!(w, style::SetForegroundColor(style::Color::Yellow), style::Print(high_score_text))?;

    queue!(w, cursor::MoveTo((width - msg.len() as u16) / 2, height / 2 + 2))?;
    queue!(w, style::SetForegroundColor(style::Color::White), style::Print(msg))?;

    w.flush()
}

// --- MAIN FUNCTION ---

fn main() -> io::Result<()> {
    let args = Args::parse();
    let mut stdout = io::stdout();

    // Setup terminal
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    // Use a closure to manage the main loop and errors, ensuring cleanup happens.
    let result = (|| {
        let mut high_score = load_high_score();

        'main_loop: loop {
            show_start_screen(&mut stdout)?;
            drain_event_queue()?;

            let mut game = Game::new(args.columns, args.lines);
            game.run(&mut stdout)?;

            // If game.run() exited but the game wasn't over, the user must have
            // pressed 'Q' to quit mid-game.
            if !game.is_game_over {
                break 'main_loop;
            }

            // Inside the main function's loop...
            if game.score > high_score {
                high_score = game.score;
                // This will now crash and show an error if saving fails.
                save_high_score(high_score)
                    .expect("ERROR: Could not save the high score file!");
}

            show_end_screen(&mut stdout, game.score, high_score)?;

            // Wait for user input on the end screen (R for restart, Q for quit).
            loop {
                if let Event::Key(key) = event::read()? {
                    // Only react to key presses to avoid double inputs.
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                drain_event_queue()?;
                                continue 'main_loop;
                            }
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                                break 'main_loop;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(())
    })(); // Immediately invoke the closure

    // Cleanup terminal
    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}