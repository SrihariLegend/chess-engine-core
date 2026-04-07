# Interfaces

## Public APIs

### Board Module

```rust
impl Board {
    pub fn new() -> Self
    pub fn from_fen(fen: &str) -> Result<Self, FenError>
    pub fn to_fen(&self) -> String
    pub fn make_move(&mut self, m: Move) -> bool
    pub fn unmake_move(&mut self, m: Move)
    pub fn generate_legal_moves(&self) -> Vec<Move>
    pub fn is_in_check(&self, color: Color) -> bool
    pub fn piece_at(&self, sq: Square) -> Option<Piece>
    pub fn game_phase(&self) -> GamePhase
}
```

### Move Generation

```rust
pub fn generate_legal_moves(board: &Board) -> Vec<Move>
pub fn generate_captures(board: &Board) -> Vec<Move>
pub fn generate_evasions(board: &Board) -> Vec<Move>
pub fn perft(board: &mut Board, depth: u32) -> u64
```

### Search

```rust
impl SearchState {
    pub fn new(params: SearchParams) -> Self
    pub fn search(&mut self, board: &mut Board) -> Option<Move>
    pub fn iterative_deepening(&mut self, board: &mut Board) -> Option<Move>
}
```

### Evaluation

```rust
pub fn evaluate(board: &Board, ctx: &GameContext) -> i16
pub fn material_balance(board: &Board) -> i16
pub fn piece_mobility(board: &Board, phase: GamePhase) -> i16
pub fn pawn_structure(board: &Board) -> i16
pub fn king_safety(board: &Board) -> i16
```

### UCI

```rust
impl UciHandler {
    pub fn new() -> Self
    pub fn run(&mut self)
    pub fn process_command(&mut self, cmd: &str) -> bool
}
```

## Trait Implementations

### Personality Trait
```rust
pub trait Personality: Send + Sync {
    fn name(&self) -> &str;
    fn weight(&self) -> f32;
    fn set_weight(&mut self, w: f32);
    fn evaluate(&self, ctx: &GameContext, base: i16) -> i16;
}
```

## Integration Points

| Component | Input | Output |
|-----------|-------|--------|
| UCI → Board | FEN string | Board state |
| UCI → Search | `go` command | Best move |
| Search → MoveGen | Board | Legal moves |
| Search → Eval | Board | Score |
| Eval → Personality | Base score, context | Modified score |
| Search → Narrative | Depth, position | Theme selection |
