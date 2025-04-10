# Rust Chess Game

A simple chess game implemented in Rust using the ggez game engine.

## Features

- Graphical chess board with piece images
- Complete chess piece movement according to rules:
  - Castling (kingside and queenside)
  - En passant captures
  - Pawn promotion to queen, rook, bishop, or knight
- Rules enforcement:
  - Check detection
  - Checkmate detection
  - Stalemate detection
  - Can't move into check
  - Can't castle through check or while in check
- Complete draw conditions:
  - Draw by threefold repetition
  - Draw by fifty-move rule
  - Draw by insufficient material
  - Draw by agreement (offer/accept)
- Chess timer with multiple time control options:
  - 5 minute blitz
  - 10 minute rapid
  - 15 minutes with 10 second increment
  - 30 minute classical
  - 60 minute classical
  - Flag falls when time expires
- Turn-based gameplay
- Visual highlighting of selected pieces and possible moves
- Game management:
  - New game button
  - Draw offer/accept buttons
  - Timer controls (start/pause and change time control)
  - Game state display

## Requirements

- Rust (latest stable version recommended)
- ggez game engine dependencies (automatically installed via Cargo)

## Running the Game

1. Clone this repository
2. Navigate to the project directory
3. Run the game using Cargo:

```
cargo run --release
```

## How to Play

- Click on a piece to select it
- Click on a highlighted square to move the selected piece
- Click on the selected piece again to deselect it
- The game automatically alternates turns between white and black
- When a pawn reaches the opposite end of the board, a promotion dialog appears
- When a king is in check, checkmate, or stalemate, the status is shown at the bottom
- Use the "Offer Draw" button to propose a draw to your opponent
- Use the "Accept Draw" button to accept a draw offer
- Use the "New Game" button to reset the board and start again
- Use the "Start Timer" button to enable the chess clock
- Use the "Change Time" button to cycle through different time control options

## Timer Controls

The game includes a chess clock with the following features:
- Choose from multiple time control options (5min, 10min, 15min+10sec, 30min, 60min)
- Start/pause timer functionality
- Visual indication of whose clock is running
- Automatic turn switching and clock management
- Flag falls when time runs out, resulting in a loss

## Draw Conditions

The game will automatically detect the following draw conditions:

- **Stalemate**: When the current player has no legal moves but is not in check
- **Threefold Repetition**: When the same position occurs three times
- **Fifty-Move Rule**: When 50 moves have been made by each player without a pawn move or capture
- **Insufficient Material**: When neither player has enough pieces to checkmate (e.g., king vs king)
- **Draw by Agreement**: When one player offers a draw and the other accepts

## Technical Details

The game structure is organized into several modules:

- `piece.rs`: Defines the chess pieces and their movement rules
- `board.rs`: Manages the game state, board representation, and rule enforcement
- `gui.rs`: Handles rendering and user interaction
- `assets.rs`: Manages loading and displaying piece images
- `main.rs`: Entry point that sets up the game window and event loop

## Future Improvements

- Game history and move notation
- Load and save functionality
- AI opponent
- Network play 