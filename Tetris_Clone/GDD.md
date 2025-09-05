Game Design Document
Project Name: Tetris Clone
Author: Khant Htay
Date: August 5, 2025
Language: Rust 1.70+
Target Platform: Terminal (cross-platform)
1. Game Overview
Genre: Puzzle / Arcade
Target Audience: Fans of classic puzzle games, players seeking reflex-based challenges
Core Idea: The player arranges falling tetromino blocks to form complete horizontal lines, which
then disappear to free space. The game ends when the stack of blocks reaches the top of the
screen.
2. Gameplay Description
Game Objective: Prevent the blocks from stacking to the top by completing horizontal lines.
Rules:
- Blocks (tetrominoes) fall from the top at a fixed speed, increasing over time.
- The player can rotate and move blocks left or right before they land.
- A complete horizontal line disappears, and the score increases.
- Game ends when new blocks can no longer spawn at the top.
Win Condition: No specific win condition — the game continues until failure (score-based).
Lose Condition: Blocks reach the top of the screen.
3. Technical Specifications
Screen Size: Fixed terminal grid (e.g., 10x20 playfield)
Controls:
- Arrow keys (← → ↓) to move/soft drop
- Up arrow to rotate
- Q to quit the game
Dependencies:
- crossterm – input and terminal rendering
- rand – tetromino randomizer- serde (optional) – high score persistence
- chrono (optional) – time-based scoring/levels
4. Game Objects
ObjectPropertiesBehaviors
TetrominoShape, Orientation, PositionFalls over time, moves left/right, rotates
GridMatrix of cells (occupied/empty)Stores landed blocks, checks line completion
PlayerScore, Level, SpeedManages inputs, tracks score
Game StatePlayfield, Current Tetromino, Next Tetromino Drives the main game loop
5. Game Flow
1. Start Screen – Tetris logo and 'Press Enter to Start'
2. Main Game Loop:
- Spawn a tetromino at the top
- Player moves/rotates it
- Piece drops every tick or faster with down key
- If it lands, add to grid, check for full lines, and clear
- Spawn next tetromino
- If spawn fails (top blocked), game over
3. End Screen – Show score and restart/quit option
6. Variables & Data Structures
Grid: [[bool; 10]; 20] – Playfield grid storing filled cells
Tetromino: Struct with shape matrix, x/y position, rotation state
Score: Integer tracking cleared lines
Level: Increases with lines cleared, affects fall speed
GameState: Enum { Running, GameOver }
7. Function Architecture
FunctionPurpose
main()Initialize and run the game loop
render()Draws the grid and current tetrominohandle_input()Processes player input
update_game()Moves pieces, checks collisions, clears lines
spawn_tetromino()Generates a new tetromino
rotate()Rotates the current tetromino
check_line_clear()Removes full lines and updates score
game_over()Handles game over state and reset prompt
8. Development Timeline
WeekGoals
Week 1 (Aug 5)Design document, tetromino logic, rendering system
Week 2 (Aug 13)Collision detection, line clearing, grid logic
Week 3 (Aug 20)Scoring system, game over state, speed scaling
Week 4 (Aug 29)Polishing, high score (optional), documentation and video
9. Optional Features (Extra Credit)
High score system using local save file
Ghost piece to preview landing spot
Hard drop feature (spacebar)
Hold piece and swap mechanic
Multiple game modes (e.g., Zen mode, Timed mode)