# Rust Chess Game

A feature-complete chess implementation written in Rust using the GGEZ game engine.

## Features

- Complete Chess Rules Implementation:
  - Standard piece movement (pawn, knight, bishop, rook, queen, king)
  - Special moves: castling (kingside/queenside), en passant, pawn promotion
  - Check, checkmate, and stalemate detection
  - Draw conditions: threefold repetition, fifty-move rule, insufficient material
  - Move validation prevents illegal moves (moving into check, etc.)

- User Interface:
  - Graphical board with chess piece images
  - Highlighted legal moves for selected pieces
  - Visual indicators for check, checkmate, and stalemate
  - Algebraic notation coordinate display
  - Promotion dialog for pawn upgrades
  - Game status display

- Network Play:
  - Host or join games over a network
  - Real-time move synchronization
  - Game listing and selection
  - Player name customization
  - In-game communication features (draw offers, resignation, rematches)

## Installation

### Prerequisites

- Rust programming language (latest stable version) - [Install Rust](https://www.rust-lang.org/tools/install)
- Required dependencies for GGEZ:
  - **macOS**: `brew install pkg-config sdl2`
  - **Linux**: `sudo apt install pkg-config libsdl2-dev libsdl2-2.0-0`
  - **Windows**: SDL2 will be automatically downloaded if needed

### Building from Source

1. Clone the repository:

```bash
git clone https://github.com/yourusername/rust-chess.git
cd rust-chess
```

2. Build and run the game:

```bash
cargo run --release
```

## Playing the Game

### Local Play

To start a local game (two players on the same computer):

```bash
cargo run --release
```

### Starting a Server (For Network Play)

To run a dedicated chess server:

```bash
cargo run --release -- --server
```

The server will listen on port 8080 by default.

### Joining a Network Game

To play over the network:

```bash
cargo run --release -- --network
```

Additional options:
- `--address <server_address>`: Connect to a specific server (default: localhost:8080)
- `--name <player_name>`: Set your display name
- `--join <game_id>`: Join a specific game directly

Example:
```bash
cargo run --release -- --network --address chessserver.example.com:8080 --name Player1
```

## Game Controls

### Board Interaction

- **Select a piece**: Left-click on a chess piece
- **Move a piece**: Left-click on a highlighted square
- **Deselect a piece**: Left-click on the selected piece again
- **See possible moves**: They're automatically highlighted after selecting a piece

### Pawn Promotion

When a pawn reaches the opposite end of the board:
1. A promotion dialog appears
2. Select the piece you want to promote to (Queen, Rook, Bishop, Knight)

### Network Play Options

- **Create a new game**: Click the "Create Game" button when in network mode
- **Join a game**: Select from the list of available games and click "Join"
- **Refresh game list**: Click the "Refresh" button to update the list of available games
- **Offer a draw**: Click the "Offer Draw" button during a game
- **Resign a game**: Click the "Resign" button to forfeit
- **Request a rematch**: Click the "Rematch" button after a game ends

## Project Structure

- `src/main.rs`: Entry point and command-line argument handling
- `src/bin/local_game.rs`: Alternative entry point for local game only
- `src/board.rs`: Game state and chess rules implementation
- `src/piece.rs`: Chess piece definitions and movement logic
- `src/gui.rs`: User interface and rendering
- `src/assets.rs`: Graphics loading and management
- `src/network.rs`: Client networking functionality
- `src/server.rs`: Multiplayer game server implementation
- `src/zobrist.rs`: Position hashing for threefold repetition detection

## Building Custom Versions

### Local Play Only Version

```bash
cargo run --release --bin local_game
```

### Optimized Release Build

```bash
cargo build --release
```

The optimized executable will be located at `target/release/chess`.

## Troubleshooting

### Common Issues

- **"Unable to connect to server"**:
  - Ensure the server is running
  - Check your network connection
  - Verify the server address and port

- **"Cannot load images"**:
  - Make sure you're running the game from the project directory
  - Verify that the `assets/images` folder exists and contains chess piece images

- **Performance Problems**:
  - Use the `--release` flag when building/running for better performance
  - Close other resource-intensive applications

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details. 