# Chess Engine

A Rust-based chess engine with UCI interface, alpha-beta search, bitboard-based board representation, and modular personality/narrative systems.

## Features

- **UCI Protocol**: Compatible with any UCI-compatible chess GUI
- **Search**: Alpha-beta search with iterative deepening, transposition table, history/killer heuristics
- **Evaluation**: Material balance, mobility, pawn structure, king safety, piece-square tables
- **Personality System**: Modular evaluation modifiers (Romantic, MomentumTracker, EntropyMaximizer, ChaosTheory, AsymmetryAddict, ZugzwangHunter)
- **Narrative Themes**: Strategic theme selection to guide search (queenside expansion, central domination, kingside attack, sacrifice preparation)
- **Property-based Testing**: Comprehensive tests using proptest

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

Then use UCI commands (e.g., `position startpos`, `go depth 10`).

## Testing

```bash
cargo test
```

## Architecture

- **Board**: Bitboard-based representation with magic bitboards for sliding pieces
- **MoveGen**: Legal move generation with special move support (castling, en passant, promotions)
- **Search**: Alpha-beta with quiescence search, iterative deepening, time management
- **Eval**: Tapered evaluation (opening/endgame phases)

## Project Structure

```
src/
├── main.rs          # Entry point
├── lib.rs           # Library interface
├── board/           # Board state, moves, FEN, magic bitboards
├── movegen/         # Legal move generation
├── search/          # Alpha-beta search, transposition table
├── eval/            # Position evaluation
├── uci/             # UCI protocol handler
├── personality/     # Evaluation modifiers
└── narrative/       # Theme-based search guidance
tests/properties/    # Property-based tests
```
