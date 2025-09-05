**
Tetris clone, terminal-based, written in Rust. “With a lil twerks and twirks.”

---

## Table of Contents

- [Overview](#overview)  
- [Installation](#installation)  
- [Usage](#usage)  
- [Controls](#controls)  
- [Saving & Loading](#saving--loading)  
- [High Score](#high-score)  
- [Code Organization](#code-organization)  
- [Development Plan](#development-plan)

---

## Overview

Terminal-based Tetris clone.  
Written in Rust.  
Classic falling-block mechanics.  
Dynamic board size; default is 10 columns × 20 rows.

---

## Installation

```sh
git clone https://github.com/Khantdotcom/Mad_tris.git
cd Mad_tris
cargo build --release
Usage
sh
Copy code
cargo run --release
Controls
Key / Input	Action
← / →	Move piece left/right
↑	Rotate piece
↓	Soft drop
Spacebar	Hard drop
P	Toggle pause
S	Save game
L	Load game
Q / Esc	Quit

Saving & Loading
Save state saved to tetris_save.json in JSON.

Load state reads from the same file.

High Score
High score stored in highscore.txt as plain text.

Updated automatically after game over if the current score exceeds the stored high score.

Code Organization & Style
Rust 2021 edition.

Indentation: 4 spaces.

Naming:

Functions/variables: snake_case

Structs/types: UpperCamelCase

Modules & Responsibilities:

Game struct: board data, active piece, logic for movement, collision, scoring.

ActivePiece: handles position, rotation state.

Serialization via serde for saving/loading state.

Screens:

show_start_screen() – Title and key prompt.

show_end_screen() – Final and high score display.

Input processing and rendering using crossterm.

Development Plan
Core gameplay: board, pieces, movement, rotation, line clearing.

Game loop: gravity, input, rendering.

UI screens: Start and end screens.

Persistence: Save/load state, high score tracking.

Polish: speeds, gravity adjustments, status messages.

License
(Optional—add license details if applicable.)

Contact
Repository owner: Khantdotcom
Feel free to open issues or pull requests.**
