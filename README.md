Installation
git clone <repo-url>
cd rust-tetris
cargo build --release

Running the Game
cargo run --release

Controls

Arrow keys → Move / Rotate / Drop

Space → Hard drop

P → Pause

S → Save

L → Load

Q → Quit

Save/Load

Game saves to tetris_save.json

High score saved in highscore.txt

Code Style

Follows Rust 2021 edition defaults

Consistent indentation (4 spaces)

Snake_case for functions and variables

UpperCamelCase for structs
