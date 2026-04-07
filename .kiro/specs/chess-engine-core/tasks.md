# Implementation Plan: chess-engine-core

## Overview

Incremental implementation of a personality-driven chess engine in Rust, organized in three phases: Core Engine (Layer 1), Personality System (Layer 2), and Narrative Layer (Layer 3). Each task builds on the previous, with property-based tests placed close to the code they validate.

## Tasks

- [x] 1. Set up project structure and core types
  - [x] 1.1 Initialize Cargo project and create module directory structure
    - Run `cargo init` with Rust 2021 edition
    - Create directories: `src/board/`, `src/movegen/`, `src/search/`, `src/eval/`, `src/uci/`, `src/personality/`, `src/narrative/`
    - Create `mod.rs` stubs for each module and wire them into `main.rs`
    - Add `proptest` as a dev-dependency in `Cargo.toml`
    - Create `tests/properties/` directory for property-based tests
    - _Requirements: 23.1, 23.2, 23.3_

  - [x] 1.2 Define core enums, structs, and move representation
    - Implement `Piece`, `Color`, `MoveFlags` (manual bitflags), `CastlingRights`, `GamePhase` enums/structs
    - Implement `Move` struct (from, to, piece, captured, promotion, flags) fitting in 8 bytes
    - Implement `UndoInfo` struct for the undo stack
    - _Requirements: 1.1, 1.2, 11.3_

- [x] 2. Board representation and FEN parsing
  - [x] 2.1 Implement `Board` struct with bitboard state
    - 12 piece bitboards (`[[u64; 6]; 2]`), occupancy arrays, side to move, castling, en passant, halfmove clock, fullmove number
    - Implement `Board::new()` for standard starting position
    - Implement `Board::game_phase()` based on remaining material
    - _Requirements: 1.1, 1.2, 1.3_

  - [x] 2.2 Implement FEN parsing and export
    - Implement `Board::from_fen(fen: &str) -> Result<Self, FenError>` with all error variants (InvalidFieldCount, InvalidRank, InvalidPiece, InvalidSideToMove, InvalidCastling, InvalidEnPassant, InvalidHalfmoveClock, InvalidFullmoveNumber)
    - Implement `Board::to_fen(&self) -> String`
    - _Requirements: 1.4, 1.5, 1.6, 1.7_

  - [x] 2.3 Write property test: FEN Round Trip (Property 1)
    - **Property 1: FEN Round Trip**
    - Implement a FEN generator producing random valid FEN strings
    - Verify `from_fen(fen).to_fen()` parsed again produces equivalent Board state
    - **Validates: Requirements 1.4, 1.6, 1.7**

  - [x] 2.4 Write property test: Invalid FEN Rejection (Property 2)
    - **Property 2: Invalid FEN Rejection**
    - Generate strings that are not valid FEN (wrong field count, bad piece chars, illegal ranks)
    - Verify `from_fen` returns `Err` with descriptive error
    - **Validates: Requirements 1.5**

- [x] 3. Zobrist hashing
  - [x] 3.1 Implement Zobrist key generation and incremental hashing
    - Implement `ZobristKeys` struct with 781 pseudorandom 64-bit keys (768 piece-square + 1 side + 4 castling + 8 en passant)
    - Use a fixed-seed PRNG for reproducibility
    - Initialize as a lazy static (manual implementation, no external crate)
    - Wire Zobrist hash into `Board` struct, compute initial hash in `Board::new()` and `Board::from_fen()`
    - _Requirements: 4.1, 4.2_

- [x] 4. Magic bitboard attack generation
  - [x] 4.1 Implement precomputed non-sliding piece attack tables
    - Compute knight attack table (64 entries) from movement rules
    - Compute king attack table (64 entries) from movement rules
    - Compute pawn attack tables (2 colors × 64 entries) from movement rules
    - _Requirements: 2.5, 2.6_

  - [x] 4.2 Implement magic bitboard tables for sliding pieces
    - Define `MagicEntry` struct (mask, magic, shift)
    - Implement `init_magic_tables()` to populate bishop and rook attack tables
    - Implement reference ray-casting function for table generation
    - Find magic numbers via brute-force trial during init
    - Implement `bishop_attacks(sq, occ)`, `rook_attacks(sq, occ)`, `queen_attacks(sq, occ)` lookup functions
    - _Requirements: 2.1, 2.2, 2.3, 2.4_

  - [x] 4.3 Write property test: Sliding Piece Attack Correctness (Property 3)
    - **Property 3: Sliding Piece Attack Correctness (Model-Based)**
    - Generate random (square, occupancy) pairs
    - Compare magic lookup result against reference ray-casting implementation
    - **Validates: Requirements 2.2, 2.3**

  - [x] 4.4 Write property test: Queen Attack Composition (Property 4)
    - **Property 4: Queen Attack Composition**
    - For random (square, occupancy), verify `queen_attacks(sq, occ) == bishop_attacks(sq, occ) | rook_attacks(sq, occ)`
    - **Validates: Requirements 2.4**

  - [x] 4.5 Write property test: Non-Sliding Piece Attack Correctness (Property 5)
    - **Property 5: Non-Sliding Piece Attack Correctness**
    - For all 64 squares, verify knight, king, and pawn attack tables match reference implementations
    - **Validates: Requirements 2.5, 2.6**

- [x] 5. Checkpoint — Board and attack generation
  - Ensure all tests pass, ask the user if questions arise.

- [x] 6. Make/unmake move operations
  - [x] 6.1 Implement `Board::make_move` and `Board::unmake_move`
    - Update all affected bitboards, occupancy, side to move, castling rights, en passant, halfmove clock, fullmove number
    - Incrementally update Zobrist hash on make (XOR out old state, XOR in new state)
    - Restore full state from `UndoInfo` on unmake
    - Handle special moves: castling (king + rook), en passant (remove captured pawn from correct square), promotion (replace pawn with promoted piece)
    - _Requirements: 11.1, 11.2, 11.3, 4.2, 4.3_

  - [x] 6.2 Write property test: Make/Unmake Round Trip (Property 8)
    - **Property 8: Make/Unmake Round Trip**
    - Generate random Board positions, pick a legal move, make then unmake
    - Verify all 12 bitboards, occupancy, side to move, castling, en passant, halfmove clock, fullmove number, and Zobrist hash are restored
    - **Validates: Requirements 4.3, 11.4**

- [x] 7. Legal move generation
  - [x] 7.1 Implement `generate_legal_moves`
    - Generate pseudo-legal moves for all piece types (pawn single/double push, captures, en passant, promotions to Q/R/B/N; knight, bishop, rook, queen, king moves; castling kingside/queenside)
    - Filter out moves that leave own king in check using `Board::is_in_check`
    - Implement `Board::is_in_check(color)` using attack tables
    - Return `MoveGenResult::Checkmate` or `MoveGenResult::Stalemate` when no legal moves exist
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_

  - [x] 7.2 Implement `generate_captures` and `generate_evasions`
    - `generate_captures`: legal capture moves + queen promotions (for quiescence search)
    - `generate_evasions`: all legal moves when in check (block, capture checker, move king)
    - _Requirements: 6.2, 6.6_

  - [x] 7.3 Implement perft and perft_divide
    - `perft(board, depth) -> u64`: count leaf nodes at given depth
    - `perft_divide(board, depth) -> Vec<(Move, u64)>`: per-move node counts
    - Add built-in perft test cases for 5 standard positions (initial, Kiwipete, positions 3-5)
    - _Requirements: 24.1, 24.2, 24.3_

  - [x] 7.4 Write property test: Generated Moves Are Legal (Property 6)
    - **Property 6: Generated Moves Are Legal**
    - Generate random Board positions, for each legal move verify making it does not leave own king in check
    - When in check, verify every generated move resolves the check
    - **Validates: Requirements 3.1, 3.4, 3.5**

  - [x] 7.5 Write property test: Terminal Position Detection (Property 7)
    - **Property 7: Terminal Position Detection**
    - For positions with zero legal moves, verify Checkmate iff king in check, Stalemate iff king not in check
    - **Validates: Requirements 3.6, 3.7**

  - [x] 7.6 Write unit tests: Perft verification
    - Run perft for all 5 built-in positions at depths up to 6
    - Verify node counts match published values
    - _Requirements: 3.8, 24.2, 24.3_

- [x] 8. Checkpoint — Move generation and perft
  - Ensure all tests pass, ask the user if questions arise.

- [x] 9. Static evaluation
  - [x] 9.1 Implement base evaluation function
    - Implement `material_balance(board)` with standard piece values (P=100, N=320, B=330, R=500, Q=900)
    - Implement piece-square tables (separate MG and EG arrays for all 6 piece types)
    - Implement `piece_square_score(board, phase)` with tapered interpolation: `(mg * phase + eg * (24 - phase)) / 24`
    - Implement `king_safety(board)`: penalize open files near king, reward pawn shelter
    - Implement `pawn_structure(board)`: penalize doubled/isolated/backward pawns, reward passed pawns
    - Implement `piece_mobility(board)`: count legal squares per piece
    - Implement `mate_score(ply)`: return `MATE_SCORE - ply`
    - Wire into `evaluate(board, game_ctx, personalities)` returning score from side-to-move perspective
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8, 8.9_

  - [x] 9.2 Write property test: Evaluation Symmetry (Property 15)
    - **Property 15: Evaluation Symmetry**
    - For random positions, verify evaluating from White's perspective equals negation of mirrored position from Black's perspective (material + PST components)
    - **Validates: Requirements 8.1**

  - [x] 9.3 Write property test: Material Balance Correctness (Property 16)
    - **Property 16: Material Balance Correctness**
    - For positions with known piece counts, verify material balance equals `white_material - black_material`
    - **Validates: Requirements 8.2**

  - [x] 9.4 Write property test: Tapered Evaluation Interpolation (Property 17)
    - **Property 17: Tapered Evaluation Interpolation**
    - Verify game phase is in [0, 24] and tapered score equals `(mg * phase + eg * (24 - phase)) / 24`
    - **Validates: Requirements 8.4**

  - [x] 9.5 Write property test: Pawn Structure Penalties (Property 18)
    - **Property 18: Pawn Structure Penalties**
    - For positions with doubled pawns, verify penalty is applied; for isolated pawns, verify isolated penalty
    - **Validates: Requirements 8.6**

  - [x] 9.6 Write property test: Mate Score Distance Encoding (Property 19)
    - **Property 19: Mate Score Distance Encoding**
    - For checkmate at ply N, verify score equals `MATE_SCORE - N`
    - **Validates: Requirements 8.8**

- [x] 10. Transposition table
  - [x] 10.1 Implement transposition table
    - Implement `TTEntry` struct (key, best_move, score, depth, node_type, age)
    - Implement `TranspositionTable` with `new(size_mb)`, `probe(hash)`, `store(hash, entry)`, `clear()`, `new_generation()`, `resize(size_mb)`
    - Replacement policy: prefer greater depth in current generation; replace older generation entries regardless of depth
    - Entry size 24 bytes, entries = `(size_mb * 1024 * 1024) / 24`
    - _Requirements: 4.4, 4.5, 4.6, 4.7_

  - [x] 10.2 Write property test: TT Store/Probe Round Trip (Property 9)
    - **Property 9: Transposition Table Store/Probe Round Trip**
    - Store a TTEntry, probe with same hash, verify fields match
    - **Validates: Requirements 4.4**

  - [x] 10.3 Write property test: TT Replacement Policy (Property 10)
    - **Property 10: Transposition Table Replacement Policy**
    - Store multiple entries to same bucket, verify retained entry follows depth/age replacement rules
    - **Validates: Requirements 4.6**

- [x] 11. Move ordering
  - [x] 11.1 Implement move ordering system
    - Implement MVV-LVA scoring: `victim_value * 10 - attacker_value`
    - Implement killer move table: 2 non-capture moves per ply
    - Implement history heuristic table: `[[i32; 64]; 12]` indexed by piece and destination square
    - Implement `order_moves` applying priority: TT best move → captures by MVV-LVA → killers → history scores
    - _Requirements: 7.1, 7.2, 7.3, 7.4_

  - [x] 11.2 Write property test: MVV-LVA Capture Ordering (Property 12)
    - **Property 12: MVV-LVA Capture Ordering**
    - For any list of captures, verify MVV-LVA scores are in non-increasing order after sorting
    - **Validates: Requirements 6.4, 7.2**

  - [x] 11.3 Write property test: Move Ordering Priority (Property 13)
    - **Property 13: Move Ordering Priority**
    - Verify TT move is first, then captures, then killers, then quiet moves by history
    - **Validates: Requirements 7.1**

  - [x] 11.4 Write property test: Killer and History Table Invariants (Property 14)
    - **Property 14: Killer and History Table Invariants**
    - Verify killer table stores at most 2 non-capture entries per ply; history scores accumulate correctly
    - **Validates: Requirements 7.3, 7.4**

- [x] 12. Alpha-beta search with iterative deepening and quiescence
  - [x] 12.1 Implement alpha-beta search with iterative deepening
    - Implement `SearchState` struct with TT, killer moves, history table, nodes counter, stop flag
    - Implement `iterative_deepening`: loop depth 1..N, call alpha_beta, report info per depth, stop on time expiry
    - Implement `alpha_beta(board, depth, alpha, beta, ply)`: TT probe, legal move generation, recursive search with negamax, TT store
    - Use PV from previous iteration for root move ordering
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

  - [x] 12.2 Implement quiescence search
    - Implement `quiescence(board, alpha, beta, ply)`: stand-pat evaluation, generate captures + queen promotions (or evasions if in check), MVV-LVA ordering, delta pruning
    - Delta pruning threshold: 900 (queen value); per-move: `stand_pat + piece_value(captured) + 200 < alpha`
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

  - [x] 12.3 Write property test: Alpha-Beta Equivalence to Minimax (Property 11)
    - **Property 11: Alpha-Beta Equivalence to Minimax**
    - For random positions at depth ≤ 4, verify alpha-beta returns same score as plain negamax
    - **Validates: Requirements 5.1**

- [x] 13. Time management
  - [x] 13.1 Implement time allocation
    - Implement `allocate_time(params)`: `remaining / moves_left + increment`, capped at 50% of remaining time
    - Handle `movetime`, `depth`, `infinite` modes
    - Default `moves_left = 30` when `movestogo` absent; use `movestogo + safety_margin` when present
    - Wire time check into search loop via `AtomicBool` stop flag
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6_

  - [x] 13.2 Write property test: Time Allocation Formula (Property 22)
    - **Property 22: Time Allocation Formula**
    - For random time control params, verify allocated time matches formula and 50% cap
    - **Validates: Requirements 10.1, 10.2**

- [x] 14. UCI protocol handler
  - [x] 14.1 Implement UCI command parsing and response
    - Implement `UciHandler` struct with Board, SearchState, UciOptions, GameContext
    - Implement `run()` main loop reading stdin line by line
    - Handle commands: `uci` (respond with id, options, uciok), `isready` (readyok), `position` (FEN + moves), `go` (start search with time params), `stop` (abort search), `quit` (exit), `ucinewgame` (clear TT, reset state), `setoption` (hash size, max depth, personality weights)
    - Handle `go perft <depth>` for perft testing
    - Output `bestmove` in long algebraic notation after search
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 9.9, 9.10, 24.1_

  - [x] 14.2 Write property test: UCI Position Command Correctness (Property 20)
    - **Property 20: UCI Position Command Correctness**
    - For valid FEN + move sequences, verify position command produces same Board as manual FEN parse + make_move
    - **Validates: Requirements 9.3**

  - [x] 14.3 Write property test: Bestmove Output Format (Property 21)
    - **Property 21: Bestmove Output Format**
    - Verify bestmove output matches `bestmove [a-h][1-8][a-h][1-8][qrbn]?`
    - **Validates: Requirements 9.6**

- [x] 15. Checkpoint — Core engine complete
  - Ensure all tests pass, ask the user if questions arise.

- [x] 16. Personality trait system and GameContext
  - [x] 16.1 Implement PersonalityEval trait, GameContext, and GameArc
    - Define `PersonalityEval` trait with `evaluate(&self, board: &Board, ctx: &GameContext) -> i32`, `weight() -> f32`, `set_weight(&mut self, w: f32)`, `name() -> &str`
    - Implement `GameContext` struct with move_number, phase, eval_history (circular buffer of 8), eval_history_len, our/their legal move counts
    - Implement `GameContext::push_eval`, `momentum()` (linear regression slope), `update_phase()`
    - Implement `GameArc` struct with default weight profiles per phase (Opening, EarlyMG, LateMG, Endgame) for 6 personalities
    - Wire personality summation into `evaluate()`: `sum(weight_i * phase_weight_i * personality_i.evaluate())`
    - _Requirements: 12.1, 12.2, 12.3, 12.4, 22.5_

  - [x] 16.2 Write property test: Weighted Personality Summation (Property 23)
    - **Property 23: Weighted Personality Summation**
    - For known weights and evaluate outputs, verify total equals `sum(weight_i * evaluate_i)`
    - **Validates: Requirements 12.2**

  - [x] 16.3 Write property test: Game Arc Phase Weight Profiles (Property 34)
    - **Property 34: Game Arc Phase Weight Profiles**
    - Verify default weight table matches specified profiles per phase
    - **Validates: Requirements 22.1, 22.2, 22.3, 22.4**

- [x] 17. Personality modules
  - [x] 17.1 Implement Chaos Theory personality
    - Implement `ChaosTheory` struct implementing `PersonalityEval`
    - Positive bonus proportional to total legal moves (both sides)
    - Negative penalty when total piece count below simplification threshold
    - _Requirements: 13.1, 13.2, 13.3, 13.4_

  - [x] 17.2 Implement Romantic personality
    - Implement `Romantic` struct implementing `PersonalityEval`
    - Positive bonus per piece proportional to squares attacked
    - Negative penalty for pieces with fewer than 3 available moves
    - _Requirements: 14.1, 14.2, 14.3, 14.4_

  - [x] 17.3 Implement Entropy Maximizer personality
    - Implement `EntropyMaximizer` struct implementing `PersonalityEval`
    - Bonus proportional to `our_legal_moves - their_legal_moves`
    - _Requirements: 15.1, 15.2, 15.3_

  - [x] 17.4 Implement Asymmetry Addict personality
    - Implement `AsymmetryAddict` struct implementing `PersonalityEval`
    - Penalty proportional to pawn file symmetry between sides
    - Bonus for material imbalances (bishop pair vs knight pair, rook vs two minors)
    - _Requirements: 16.1, 16.2, 16.3, 16.4_

  - [x] 17.5 Implement Momentum Tracker personality
    - Implement `MomentumTracker` struct implementing `PersonalityEval`
    - Compute momentum from eval_history trend (linear regression slope)
    - Positive bonus for improving trend, negative penalty for worsening trend
    - _Requirements: 17.1, 17.2, 17.3, 17.4, 17.5_

  - [x] 17.6 Implement Zugzwang Hunter personality
    - Implement `ZugzwangHunter` struct implementing `PersonalityEval`
    - Bonus inversely proportional to opponent's legal move count (clamp denominator to min 1)
    - Increased weight multiplier during Endgame phase
    - _Requirements: 18.1, 18.2, 18.3, 18.4_

  - [x] 17.7 Write property tests for personality modules (Properties 24-29)
    - **Property 24: Chaos Theory Monotonicity** — more total legal moves → higher bonus; below threshold → negative penalty
    - **Validates: Requirements 13.2, 13.3**
    - **Property 25: Romantic Activity Scoring** — bonus proportional to squares attacked; penalty for pieces with <3 moves
    - **Validates: Requirements 14.2, 14.3**
    - **Property 26: Entropy Maximizer Proportionality** — bonus proportional to `our_moves - their_moves`
    - **Validates: Requirements 15.2**
    - **Property 27: Asymmetry Addict Scoring** — penalty proportional to pawn symmetry; bonus for material imbalances
    - **Validates: Requirements 16.2, 16.3**
    - **Property 28: Momentum Tracker Trend Alignment** — output sign matches eval trend direction
    - **Validates: Requirements 17.2, 17.3, 17.4**
    - **Property 29: Zugzwang Hunter Inverse Proportionality** — fewer opponent moves → higher bonus; endgame weight > non-endgame
    - **Validates: Requirements 18.2, 18.3**

- [x] 18. Checkpoint — Personality system complete
  - Ensure all tests pass, ask the user if questions arise.

- [x] 19. Narrative layer
  - [x] 19.1 Implement Storyteller module
    - Implement `Storyteller::select_theme(board, move_number) -> Option<Theme>`: select theme between moves 3-5, return None for moves 1-2
    - Implement theme selection heuristics (KingsideAttack, CentralDomination, QueensideExpansion, PieceSacrificePreparation) based on position features
    - Implement `evaluate_move_alignment(board, mv, theme) -> i32`: positive for thematic moves, negative for contradicting
    - Define `Theme` enum and `NarrativeContext` struct
    - _Requirements: 19.1, 19.2, 19.3, 19.4_

  - [x] 19.2 Implement Time Traveler module
    - Implement `TimeTraveler::depth_extension(board, mv, narrative_ctx) -> i32`
    - Return positive extension (default 1 ply) for theme-aligned moves, 0 otherwise
    - Track and enforce `max_depth_extensions` limit per search
    - _Requirements: 20.1, 20.2, 20.3_

  - [x] 19.3 Implement Mirage module
    - Implement `Mirage::deception_bonus(board, mv, theme) -> i32`
    - Count distinct strategic purposes a move serves (attacking, defending, center control, opening files, pawn structure, piece activity, restricting mobility)
    - Bonus proportional to purposes beyond the active theme; zero/minimal for single-purpose moves
    - _Requirements: 21.1, 21.2, 21.3_

  - [x] 19.4 Write property test: Storyteller Theme Selection Timing (Property 30)
    - **Property 30: Storyteller Theme Selection Timing**
    - Verify `Some(Theme)` returned at moves 3-5, `None` at moves 1-2
    - **Validates: Requirements 19.1**

  - [x] 19.5 Write property test: Theme Alignment Scoring (Property 31)
    - **Property 31: Theme Alignment Scoring**
    - Verify positive score for thematic moves, negative for contradicting moves
    - **Validates: Requirements 19.2, 19.3**

  - [x] 19.6 Write property test: Time Traveler Depth Extension (Property 32)
    - **Property 32: Time Traveler Depth Extension**
    - Verify positive extension for aligned moves, 0 for non-aligned; cumulative extensions ≤ max
    - **Validates: Requirements 20.1, 20.2**

  - [x] 19.7 Write property test: Mirage Deception Bonus (Property 33)
    - **Property 33: Mirage Deception Bonus**
    - Verify positive bonus for multi-purpose moves, zero/minimal for single-purpose
    - **Validates: Requirements 21.1, 21.2**

- [x] 20. Wire narrative layer into search
  - [x] 20.1 Integrate narrative context into search loop
    - Add `NarrativeContext` to `SearchState`
    - Call `Storyteller::select_theme` at appropriate move numbers during `handle_go`
    - Apply `TimeTraveler::depth_extension` in alpha-beta loop for theme-aligned moves
    - Add `Mirage::deception_bonus` to evaluation for moves with active theme
    - Wire `GameArc` phase weights into personality evaluation during search
    - _Requirements: 19.1, 19.2, 20.1, 21.1, 22.1, 22.2, 22.3, 22.4_

- [x] 21. Wire main.rs entry point
  - [x] 21.1 Implement main.rs with CLI and UCI loop
    - Call `init_magic_tables()` at startup
    - Parse CLI args: support `--perft <depth> [fen]` for standalone perft testing
    - Default to UCI mode: create `UciHandler` and call `run()`
    - _Requirements: 23.1, 24.1_

- [x] 22. Final checkpoint — Full engine integration
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation after major milestones
- Property tests validate universal correctness properties from the design document (34 total)
- Perft tests are critical for move generation correctness — run them early and often
- No external crates for core logic; `proptest` is allowed as dev-dependency only
