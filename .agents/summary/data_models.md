# Data Models

## Core Types

### Board State
```rust
pub struct Board {
    pieces: [Bitboard; 12],  // 6 piece types × 2 colors
    occupancy: [Bitboard; 2], // white/black occupancy
    side_to_move: Color,
    castling_rights: CastlingRights,
    en_passant: Option<Square>,
    halfmove_clock: u32,
    fullmove_number: u32,
}
```

### Move Encoding
```rust
pub struct Move {
    from: Square,      // 6 bits (0-63)
    to: Square,        // 6 bits (0-63)
    flags: MoveFlags,  // 4 bits
    // Total: 16 bits, fits in u16
}

pub enum MoveFlags {
    Quiet,
    Capture,
    DoublePawnPush,
    KingCastle,
    QueenCastle,
    Promotion(u8),     // N, B, R, Q
    EnPassant,
}
```

### Castling Rights
```rust
pub struct CastlingRights {
    white_kingside: bool,
    white_queenside: bool,
    black_kingside: bool,
    black_queenside: bool,
}
```

### Game Phase
```rust
pub enum GamePhase {
    Opening,
    EarlyMiddlegame,
    LateMiddlegame,
    Endgame,
}
```

### Search Types
```rust
pub struct SearchParams {
    depth: u32,
    movetime: Option<u64>,
    nodes: Option<u64>,
    infinite: bool,
    // ... time control params
}

pub struct SearchInfo {
    nodes_searched: u64,
    best_score: i16,
    pv: Vec<Move>,
    // ... stats
}

pub struct TTEntry {
    hash: u64,
    depth: u8,
    score: i16,
    node_type: NodeType,
    best_move: Option<Move>,
    generation: u8,
}

pub enum NodeType {
    Pv,    // Exact score
    Cut,   // Beta cutoff
    All,   // All nodes
}
```

### Evaluation Context
```rust
pub struct GameContext {
    phase: GamePhase,
    move_number: u32,
    eval_history: Vec<i16>,  // Circular buffer
    // ... phase boundaries
}

pub struct NarrativeContext {
    theme: Theme,
    depth: u32,
    // ... theme-specific data
}

pub enum Theme {
    None,
    QueensideExpansion,
    CentralDomination,
    KingsideAttack,
    SacrificePrep,
}
```

### Personality Types
```rust
pub struct GameArc {
    opening_end: u32,
    early_middlegame_end: u32,
    late_middlegame_end: u32,
    // Phase boundaries in move pairs
}
```

## Bitboard Types

```rust
pub type Bitboard = u64;

pub struct Square(u8);  // 0-63
pub struct Color(u8);   // 0=white, 1=black
pub struct Piece(u8);   // 0-11 (P,N,B,R,Q,K for white/black)
```

## UCI Types

```rust
pub struct UciOptions {
    hash: u32,          // TT size in MB
    maxdepth: u32,
    threads: u32,
    // ... UCI options
}
```
