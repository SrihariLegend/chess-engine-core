# Chess Engine

A Rust-based chess engine with UCI interface, alpha-beta search, bitboard-based board representation, and modular personality system.

## Features

- **UCI Protocol**: Compatible with any UCI-compatible chess GUI
- **Search**: Alpha-beta search with iterative deepening, transposition table, history/killer heuristics
- **Evaluation**: Material balance, mobility, pawn structure, king safety, piece-square tables
- **Personality System**: Modular evaluation modifiers (Romantic, MomentumTracker, EntropyMaximizer, ChaosTheory, AsymmetryAddict, ZugzwangHunter)
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

Property-based tests are located in `tests/properties/` using `proptest`:
- `board_props.rs` - FEN encoding/decoding and move round-trip tests
- `movegen_props.rs` - Perft verification tests
- `eval_props.rs` - Evaluation sanity tests
- `search_props.rs` - Search correctness tests

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
├── personality/     # Evaluation modifiers (6 implementations)
└── narrative/       # (removed - not implemented)
tests/properties/    # Property-based tests
```

## Key Components

| Component | File | Purpose |
|-----------|------|---------|
| Board | `src/board/mod.rs` | Bitboard state, move application, FEN |
| Magic | `src/board/magic.rs` | Sliding piece attack tables |
| MoveGen | `src/movegen/mod.rs` | Legal move generation |
| Search | `src/search/mod.rs` | Alpha-beta, iterative deepening |
| TT | `src/search/tt.rs` | Transposition table |
| Eval | `src/eval/mod.rs` | Material, mobility, pawns, king safety |
| UCI | `src/uci/mod.rs` | UCI protocol commands |

## Personality System

The engine supports 6 different evaluation personalities that modify the base evaluation:

- **Romantic** - Prefers active, aggressive play with piece activity bonuses
- **MomentumTracker** - Tracks evaluation trends and adapts based on position momentum
- **EntropyMaximizer** - Favors moves that increase move count differences, creating imbalance
- **ChaosTheory** - Simplifies positions by penalizing material complexity
- **AsymmetryAddict** - Seeks asymmetrical board positions and unequal pawn structures
- **ZugzwangHunter** - Specializes in endgame positions, counting available moves

Located in `src/personality/`.

## Entry Points

- **CLI**: `src/main.rs` → `main()` - Command-line interface entry point
- **UCI loop**: `src/uci/mod.rs` → `UciHandler::run()` - UCI protocol handler main loop
- **Search**: `src/search/mod.rs` → `SearchState::search()` - Core search algorithm
