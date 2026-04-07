# Components

## Core Modules

### src/board/mod.rs (2112 LOC)
**Purpose**: Board state management and move application

**Key Types**:
- `Board` - Main game state with piece placement, turn, castling rights, en passant
- `Move` - Move encoding (from, to, flags)
- `CastlingRights` - Castling availability tracking
- `ZobristKeys` - Position hashing
- `FenError` - FEN parsing errors

**Key Functions**:
- `from_fen()` / `to_fen()` - FEN import/export
- `make_move()` / `unmake_move()` - Move application
- `is_in_check()` - Check detection
- `generate_legal_moves()` - Legal move generation

### src/board/magic.rs (540 LOC)
**Purpose**: Attack table generation using magic bitboards

**Key Types**:
- `AttackTables` - Precomputed attack bitboards
- `MagicEntry` - Magic number entries for sliding pieces
- `SlidingTables` - Sliding piece attack tables

**Key Functions**:
- `init_magic_tables()` - Initialize attack tables
- `bishop_attacks()` / `rook_attacks()` / `queen_attacks()`
- `knight_attacks()` / `king_attacks()` / `pawn_attacks()`

### src/movegen/mod.rs (809 LOC)
**Purpose**: Legal move generation

**Key Types**:
- `MoveGenResult` - Container for generated moves

**Key Functions**:
- `generate_legal_moves()` - Get all legal moves
- `generate_captures()` - Capture-only moves
- `generate_evasions()` - Moves from check
- `perft()` - Move counting for testing

### src/search/mod.rs (980 LOC)
**Purpose**: Search algorithm implementation

**Key Types**:
- `SearchState` - Search state (TT, history, killers)
- `SearchInfo` - Search statistics
- `SearchParams` - Search configuration
- `HistoryTable` - History heuristic
- `KillerTable` - Killer move table

**Key Functions**:
- `iterative_deepening()` - Main search with time control
- `alpha_beta()` - Alpha-beta pruning
- `quiescence()` - Quiescence search
- `order_moves()` - Move ordering

### src/search/tt.rs (317 LOC)
**Purpose**: Transposition table

**Key Types**:
- `TranspositionTable` - Hash table for search results
- `TTEntry` - Single table entry
- `NodeType` - PV, cut, or all node

**Key Functions**:
- `probe()` / `store()` - TT access
- `resize()` - Resize table

### src/eval/mod.rs (622 LOC)
**Purpose**: Position evaluation

**Key Functions**:
- `evaluate()` - Main evaluation
- `material_balance()` - Piece values
- `piece_mobility()` - Mobility scoring
- `pawn_structure()` - Pawn structure analysis
- `king_safety()` - King safety evaluation

### src/uci/mod.rs (522 LOC)
**Purpose**: UCI protocol implementation

**Key Types**:
- `UciHandler` - UCI command processor
- `UciOptions` - Configurable options

**Key Functions**:
- `run()` - Main UCI loop
- `process_command()` - Command dispatcher
- `handle_position()` / `handle_go()` - UCI commands

### src/personality/mod.rs (433 LOC)
**Purpose**: Personality evaluation system

**Key Types**:
- `GameContext` - Phase and history tracking
- `GameArc` - Phase boundaries
- `MockPersonality` - Test trait implementation

**Key Functions**:
- `personality_score()` - Combined personality evaluation
- `update_phase()` - Phase transition detection
- `momentum()` - Eval trend calculation

### src/narrative/mod.rs (47 LOC)
**Purpose**: Theme context

**Key Types**:
- `NarrativeContext` - Theme selection context
- `Theme` - Strategic theme enum

## Personality Implementations

| File | Type | Purpose |
|------|------|---------|
| `romantic.rs` | Romantic | Active piece bonus |
| `momentum_tracker.rs` | MomentumTracker | Eval trend tracking |
| `entropy_maximizer.rs` | EntropyMaximizer | Move count differences |
| `chaos_theory.rs` | ChaosTheory | Simplification scoring |
| `asymmetry_addict.rs` | AsymmetryAddict | Board asymmetry |
| `zugzwang_hunter.rs` | ZugzwangHunter | Endgame move counting |

## Narrative Implementations

| File | Type | Purpose |
|------|------|---------|
| `storyteller.rs` | Storyteller | Theme selection |
| `mirage.rs` | Mirage | Strategic purpose |
| `time_traveler.rs` | TimeTraveler | Depth extensions |
