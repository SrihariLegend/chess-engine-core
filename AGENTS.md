# AGENTS.md - AI Assistant Guide

## Project Overview

A Rust-based chess engine with UCI interface, alpha-beta search, bitboard-based board representation, and modular personality system.

## Directory Structure

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

## Entry Points

- **CLI**: `src/main.rs` → `main()`
- **UCI loop**: `src/uci/mod.rs` → `UciHandler::run()`
- **Search**: `src/search/mod.rs` → `SearchState::search()`

## Personality System

Located in `src/personality/`:
- `Romantic` - Active piece bonus
- `MomentumTracker` - Eval trend tracking
- `EntropyMaximizer` - Move count differences
- `ChaosTheory` - Simplification scoring
- `AsymmetryAddict` - Board asymmetry
- `ZugzwangHunter` - Endgame move counting

## Testing

Property-based tests in `tests/properties/` using `proptest`:
- `board_props.rs` - FEN, move round-trip
- `movegen_props.rs` - Perft verification
- `eval_props.rs` - Evaluation sanity
- `search_props.rs` - Search correctness

## Custom Instructions

<!-- This section is for human and agent-maintained operational knowledge.
     Add repo-specific conventions, gotchas, and workflow rules here.
     This section is preserved exactly as-is when re-running codebase-summary. -->
