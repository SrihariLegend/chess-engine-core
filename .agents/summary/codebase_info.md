# Codebase Information

## Basic Details

- **Workspace Path**: `/home/tom/codespace/engine`
- **Language**: Rust
- **Total Files**: 38
- **Lines of Code**: ~11,604
- **Size Category**: S (Small)

## Technology Stack

- **Language**: Rust
- **Build System**: Cargo
- **Testing**: Property-based testing with `proptest`

## Package Structure

| Package | Description | LOC Score |
|---------|-------------|-----------|
| `src/board` | Board state, moves, FEN parsing, bitboards | 582.7 |
| `src/movegen` | Legal move generation, perft | 207.7 |
| `src/search` | Alpha-beta search, time control, TT | 188.4 |
| `src/uci` | UCI protocol interface | 136.7 |
| `src/eval` | Position evaluation | 102.7 |
| `src/personality` | Evaluation modifiers | 102.1 |
| `src/narrative` | Theme-based move scoring | 50.0 |
| `tests/properties` | Property-based tests | ~30 |

## Key Architectural Patterns

- **Bitboard-based**: Uses bitboards for piece placement and attack computation
- **Magic bitboards**: Sliding piece attack generation
- **Transposition table**: Caches search results
- **Personality system**: Modular evaluation modifiers
- **Narrative themes**: Strategic move alignment in search
