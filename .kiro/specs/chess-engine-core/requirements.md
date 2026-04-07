# Requirements Document

## Introduction

A chess engine built in Rust with a personality-driven architecture organized in three layers: a high-performance core engine (Phase 1), personality-based evaluation modifiers (Phase 2), and a narrative meta-planning layer (Phase 3). The engine's philosophical identity is "Every game is a story that was always going to end this way." Target playing strength is ~2600 Elo after full tuning, with a ~2200 Elo baseline from the core engine alone.

The engine uses bitboard representation with magic bitboards for sliding pieces, alpha-beta search with iterative deepening, Zobrist hashing for transposition tables, and communicates via the UCI protocol. Personality traits are weighted f32 evaluation terms implementing a common `PersonalityEval` trait. The narrative layer operates as a meta-planner above the search, selecting themes and adjusting search behavior across defined game arcs.

## Glossary

- **Engine**: The chess engine binary, the complete system including all layers
- **Board**: The internal board representation using 64-bit bitboards for each piece type and color
- **Bitboard**: A 64-bit unsigned integer where each bit represents one square on the chess board
- **Magic_Bitboard**: A precomputed lookup table using magic number multiplication to generate sliding piece attack sets in constant time
- **Move_Generator**: The subsystem responsible for producing all legal moves from a given Board position
- **Legal_Move**: A move that does not leave the moving side's king in check
- **Search**: The alpha-beta search subsystem with iterative deepening that explores the game tree to select moves
- **Transposition_Table**: A hash table keyed by Zobrist hash that caches previously evaluated positions to avoid redundant search
- **Zobrist_Hash**: A 64-bit hash of a board position computed by XOR-ing precomputed random values for each piece-square combination, side to move, castling rights, and en passant file
- **Quiescence_Search**: An extension of the main search that continues searching capture sequences beyond the main search depth to avoid horizon effects
- **MVV_LVA**: Most Valuable Victim / Least Valuable Attacker, a move ordering heuristic that prioritizes capturing high-value pieces with low-value pieces
- **UCI_Handler**: The subsystem that parses and responds to Universal Chess Interface protocol messages
- **UCI**: Universal Chess Interface, a standard protocol for communication between chess engines and GUIs
- **Evaluator**: The subsystem that assigns a numerical score (in centipawns) to a board position
- **Centipawn**: A unit of evaluation equal to 1/100th of a pawn's value
- **Perft**: Performance test that counts the number of leaf nodes at a given depth to verify move generation correctness
- **PersonalityEval**: A Rust trait that personality modules implement, providing an `evaluate` method and a `weight` method
- **GameContext**: A struct carrying game state metadata including move history, eval trend, move number, and game phase
- **Game_Phase**: A classification of the current position as Opening (moves 1-10), Early_Middlegame (moves 10-20), Late_Middlegame (moves 20-30), or Endgame (move 30+)
- **Chaos_Theory**: A personality trait that rewards board complexity and penalizes simplification
- **Romantic**: A personality trait that rewards piece mobility and activity, penalizing passive pieces
- **Entropy_Maximizer**: A personality trait that rewards asymmetry in legal move counts between sides
- **Asymmetry_Addict**: A personality trait that penalizes symmetric pawn structures and rewards material/positional imbalances
- **Momentum_Tracker**: A personality trait that tracks the evaluation trend over the last 8 moves and adjusts aggression accordingly
- **Zugzwang_Hunter**: A personality trait that rewards positions reducing the opponent's good moves, especially in endgames
- **Storyteller**: A narrative module that selects a thematic plan between moves 3-5 and evaluates subsequent moves for alignment with that theme
- **Time_Traveler**: A narrative module that grants extra search depth to moves aligned with the Storyteller's chosen theme
- **Mirage**: A narrative module that rewards moves concealing the engine's strategic intention from the opponent
- **Game_Arc**: The phased personality weighting system that adjusts which personality traits are active based on Game_Phase
- **Theme**: A strategic plan selected by the Storyteller (e.g., kingside attack, central domination, piece sacrifice preparation)

## Requirements

### Requirement 1: Bitboard Board Representation

**User Story:** As a chess engine developer, I want the board state represented using 64-bit bitboards, so that move generation and evaluation can leverage fast bitwise operations.

#### Acceptance Criteria

1. THE Board SHALL represent each piece type and color as a separate 64-bit unsigned integer bitboard (12 bitboards total: one per piece type per color).
2. THE Board SHALL maintain additional state for side to move, castling rights (kingside and queenside for both colors), en passant target square, halfmove clock, and fullmove number.
3. THE Board SHALL initialize to the standard chess starting position when no FEN string is provided.
4. WHEN a valid FEN string is provided, THE Board SHALL parse the FEN and set the position accordingly.
5. IF an invalid FEN string is provided, THEN THE Board SHALL return a descriptive error indicating the nature of the FEN parsing failure.
6. THE Board SHALL provide a method to export the current position as a valid FEN string.
7. FOR ALL valid Board positions, parsing a FEN string then exporting to FEN then parsing again SHALL produce an equivalent Board state (round-trip property).

---

### Requirement 2: Magic Bitboard Attack Generation

**User Story:** As a chess engine developer, I want precomputed magic bitboard lookup tables for sliding pieces, so that attack set generation runs in constant time.

#### Acceptance Criteria

1. THE Magic_Bitboard module SHALL precompute attack tables for bishops and rooks at engine initialization.
2. WHEN given a square and an occupancy bitboard, THE Magic_Bitboard module SHALL return the correct attack bitboard for a bishop on that square in constant time via table lookup.
3. WHEN given a square and an occupancy bitboard, THE Magic_Bitboard module SHALL return the correct attack bitboard for a rook on that square in constant time via table lookup.
4. THE Magic_Bitboard module SHALL generate queen attacks by combining the bishop and rook attack bitboards for the given square and occupancy.
5. THE Magic_Bitboard module SHALL precompute attack tables for knights and kings (non-sliding pieces) indexed by square.
6. THE Magic_Bitboard module SHALL precompute pawn attack tables for both colors indexed by square.

---

### Requirement 3: Legal Move Generation

**User Story:** As a chess engine developer, I want fully legal move generation, so that the search tree only contains valid positions.

#### Acceptance Criteria

1. WHEN given a Board position, THE Move_Generator SHALL produce all legal moves for the side to move.
2. THE Move_Generator SHALL generate pawn moves including single push, double push, en passant capture, and promotion to queen, rook, bishop, or knight.
3. THE Move_Generator SHALL generate castling moves (kingside and queenside) only when the king and relevant rook have not moved, no squares between them are occupied, and the king does not pass through or land on an attacked square.
4. THE Move_Generator SHALL exclude any move that leaves the moving side's king in check.
5. WHEN the side to move is in check, THE Move_Generator SHALL produce only moves that resolve the check (block, capture the checking piece, or move the king).
6. WHEN the side to move has no legal moves and the king is in check, THE Move_Generator SHALL indicate checkmate.
7. WHEN the side to move has no legal moves and the king is not in check, THE Move_Generator SHALL indicate stalemate.
8. THE Move_Generator SHALL pass standard perft test suites (initial position, Kiwipete, and at least 3 additional perft positions) matching published node counts at depths up to 6.

---

### Requirement 4: Zobrist Hashing and Transposition Table

**User Story:** As a chess engine developer, I want Zobrist hashing and a transposition table, so that the search avoids re-evaluating previously seen positions.

#### Acceptance Criteria

1. THE Engine SHALL generate a set of pseudorandom 64-bit Zobrist keys at initialization: one for each piece-type/color/square combination (12 × 64 = 768 keys), one for side to move, four for castling rights, and eight for en passant files.
2. THE Board SHALL maintain an incrementally updated Zobrist_Hash that is XOR-updated on each make/unmake move operation.
3. WHEN a move is made and then unmade, THE Board SHALL restore the Zobrist_Hash to the value before the move was made.
4. THE Transposition_Table SHALL store entries keyed by Zobrist_Hash containing: best move, evaluation score, search depth, node type (exact, lower bound, upper bound), and age.
5. WHEN the Search encounters a position whose Zobrist_Hash matches a Transposition_Table entry with equal or greater depth, THE Search SHALL use the stored evaluation to prune or narrow the alpha-beta window.
6. WHEN the Transposition_Table is full, THE Transposition_Table SHALL replace entries using a replacement scheme that considers depth and age.
7. THE Transposition_Table SHALL accept a configurable size parameter in megabytes via UCI option.

---

### Requirement 5: Alpha-Beta Search with Iterative Deepening

**User Story:** As a chess engine developer, I want an alpha-beta search with iterative deepening, so that the engine finds strong moves within time constraints.

#### Acceptance Criteria

1. THE Search SHALL implement alpha-beta pruning to eliminate branches that cannot influence the final decision.
2. THE Search SHALL use iterative deepening, searching depth 1 first, then depth 2, and so on, until time runs out or the maximum depth is reached.
3. WHEN the allocated search time expires, THE Search SHALL return the best move found in the last fully completed iteration.
4. THE Search SHALL use the principal variation from the previous iteration to order moves at the root for the next iteration.
5. THE Search SHALL support a configurable maximum search depth via UCI option.
6. THE Search SHALL report search information (depth, score, nodes searched, principal variation, time elapsed, nodes per second) via UCI `info` strings during iterative deepening.

---

### Requirement 6: Quiescence Search

**User Story:** As a chess engine developer, I want quiescence search at leaf nodes, so that the engine does not evaluate positions in the middle of tactical sequences.

#### Acceptance Criteria

1. WHEN the main Search reaches depth zero, THE Search SHALL enter Quiescence_Search instead of returning a static evaluation.
2. THE Quiescence_Search SHALL evaluate captures and queen promotions to resolve tactical sequences.
3. THE Quiescence_Search SHALL use a stand-pat score (static evaluation of the current position) as a lower bound to allow early cutoffs.
4. THE Quiescence_Search SHALL apply MVV_LVA ordering to capture moves.
5. THE Quiescence_Search SHALL apply delta pruning to skip captures that cannot raise the score above alpha even with the most optimistic material gain.
6. WHEN the side to move is in check during Quiescence_Search, THE Quiescence_Search SHALL generate and search all evasion moves (not just captures).

---

### Requirement 7: Move Ordering

**User Story:** As a chess engine developer, I want effective move ordering, so that alpha-beta pruning achieves near-optimal cutoff rates.

#### Acceptance Criteria

1. THE Search SHALL order moves using the following priority (highest to lowest): transposition table best move, captures ordered by MVV_LVA score, killer moves (two per ply), history heuristic scores for quiet moves.
2. THE MVV_LVA ordering SHALL assign higher priority to captures of more valuable pieces by less valuable pieces.
3. THE Search SHALL maintain a killer move table storing up to two non-capture moves per ply that caused beta cutoffs.
4. THE Search SHALL maintain a history heuristic table that increments scores for quiet moves causing beta cutoffs, indexed by piece type and destination square.

---

### Requirement 8: Static Evaluation

**User Story:** As a chess engine developer, I want a static evaluation function, so that leaf positions receive meaningful scores for the search to compare.

#### Acceptance Criteria

1. THE Evaluator SHALL return a score in centipawns from the perspective of the side to move.
2. THE Evaluator SHALL compute material balance using standard piece values (pawn=100, knight=320, bishop=330, rook=500, queen=900).
3. THE Evaluator SHALL apply piece-square tables for all piece types, with separate tables for middlegame and endgame phases.
4. THE Evaluator SHALL compute a Game_Phase value based on remaining material to interpolate between middlegame and endgame piece-square table scores (tapered evaluation).
5. THE Evaluator SHALL evaluate king safety by penalizing open files near the king and rewarding pawn shelter.
6. THE Evaluator SHALL evaluate pawn structure by penalizing doubled pawns, isolated pawns, and backward pawns, and rewarding passed pawns.
7. THE Evaluator SHALL evaluate piece mobility by counting the number of legal squares available to each piece.
8. WHEN the position is checkmate, THE Evaluator SHALL return a score indicating mate with the distance to mate encoded (preferring shorter mates).
9. WHEN the position is stalemate, THE Evaluator SHALL return a score of zero.

---

### Requirement 9: UCI Protocol Support

**User Story:** As a chess engine user, I want UCI protocol compliance, so that the engine works with standard chess GUIs like Arena and Cute Chess.

#### Acceptance Criteria

1. WHEN the UCI_Handler receives the `uci` command, THE UCI_Handler SHALL respond with `id name`, `id author`, supported UCI options, and `uciok`.
2. WHEN the UCI_Handler receives the `isready` command, THE UCI_Handler SHALL respond with `readyok` after completing any pending initialization.
3. WHEN the UCI_Handler receives a `position` command with a FEN string and optional move list, THE UCI_Handler SHALL set the Board to the specified position and apply the moves.
4. WHEN the UCI_Handler receives a `position startpos` command with an optional move list, THE UCI_Handler SHALL set the Board to the initial position and apply the moves.
5. WHEN the UCI_Handler receives a `go` command with time control parameters (wtime, btime, winc, binc, movestogo, depth, movetime, infinite), THE UCI_Handler SHALL start the Search with appropriate time management.
6. WHEN the Search completes, THE UCI_Handler SHALL output `bestmove` followed by the selected move in long algebraic notation.
7. WHEN the UCI_Handler receives the `stop` command during search, THE UCI_Handler SHALL stop the Search and output the best move found so far.
8. WHEN the UCI_Handler receives the `quit` command, THE Engine SHALL terminate cleanly.
9. WHEN the UCI_Handler receives the `ucinewgame` command, THE Engine SHALL clear the Transposition_Table and reset game-specific state.
10. THE UCI_Handler SHALL expose configurable options for Transposition_Table size (in MB) and maximum search depth via `option` commands.

---

### Requirement 10: Time Management

**User Story:** As a chess engine user, I want intelligent time management, so that the engine allocates time appropriately across the game.

#### Acceptance Criteria

1. WHEN the `go` command includes `wtime` and `btime`, THE Search SHALL allocate a base time for the current move calculated as remaining time divided by an estimated number of moves remaining (default 30), plus the increment.
2. WHEN the `go` command includes `movestogo`, THE Search SHALL divide remaining time by `movestogo` plus a safety margin to allocate time for the current move.
3. WHEN the `go` command includes `movetime`, THE Search SHALL search for exactly the specified duration.
4. WHEN the `go` command includes `depth`, THE Search SHALL search to exactly the specified depth regardless of time.
5. WHEN the `go` command includes `infinite`, THE Search SHALL search until a `stop` command is received.
6. THE Search SHALL abort the current iteration if the elapsed time exceeds the allocated time for the current move.

---

### Requirement 11: Make/Unmake Move Operations

**User Story:** As a chess engine developer, I want efficient make and unmake move operations, so that the search can traverse the game tree without copying the entire board state.

#### Acceptance Criteria

1. WHEN a legal move is applied via the make operation, THE Board SHALL update all affected bitboards, side to move, castling rights, en passant state, halfmove clock, fullmove number, and Zobrist_Hash.
2. WHEN a move is reversed via the unmake operation, THE Board SHALL restore the Board to the exact state before the make operation, including all bitboards, side to move, castling rights, en passant state, halfmove clock, fullmove number, and Zobrist_Hash.
3. THE Board SHALL handle special moves in make/unmake: castling (moving both king and rook), en passant (removing the captured pawn from the correct square), and promotion (replacing the pawn with the promoted piece).
4. FOR ALL legal moves from any valid position, making then unmaking a move SHALL produce a Board state identical to the state before the move was made.

---

### Requirement 12: Personality Evaluation Trait System (Phase 2)

**User Story:** As a chess engine developer, I want a common trait for personality evaluation modules, so that personality-driven eval terms are modular and composable.

#### Acceptance Criteria

1. THE Engine SHALL define a `PersonalityEval` trait with an `evaluate` method accepting a Board reference and a GameContext reference, returning an i32 score, and a `weight` method returning an f32 multiplier.
2. THE Evaluator SHALL sum the weighted outputs of all active PersonalityEval implementations and add the result to the base static evaluation score.
3. THE Engine SHALL allow each PersonalityEval weight to be configured at runtime via UCI options.
4. THE GameContext SHALL contain the current move number, Game_Phase, evaluation history (last 8 evaluations), and legal move counts for both sides.

---

### Requirement 13: Chaos Theory Personality (Phase 2)

**User Story:** As a chess engine designer, I want a Chaos Theory personality trait, so that the engine rewards complex positions and avoids simplification.

#### Acceptance Criteria

1. THE Chaos_Theory module SHALL implement the PersonalityEval trait.
2. THE Chaos_Theory module SHALL assign a positive evaluation bonus proportional to the total number of legal moves available to both sides combined.
3. THE Chaos_Theory module SHALL assign a negative evaluation penalty when the total piece count drops below a threshold (discouraging trades that simplify the position).
4. THE Chaos_Theory module SHALL reside in its own Rust module file.

---

### Requirement 14: Romantic Personality (Phase 2)

**User Story:** As a chess engine designer, I want a Romantic personality trait, so that the engine rewards active, mobile pieces and penalizes passive ones.

#### Acceptance Criteria

1. THE Romantic module SHALL implement the PersonalityEval trait.
2. THE Romantic module SHALL assign a positive evaluation bonus for each piece proportional to the number of squares that piece attacks.
3. THE Romantic module SHALL assign a negative evaluation penalty for pieces that have fewer than 3 available moves (passive pieces).
4. THE Romantic module SHALL reside in its own Rust module file.

---

### Requirement 15: Entropy Maximizer Personality (Phase 2)

**User Story:** As a chess engine designer, I want an Entropy Maximizer personality trait, so that the engine rewards positions where the engine has more options than the opponent.

#### Acceptance Criteria

1. THE Entropy_Maximizer module SHALL implement the PersonalityEval trait.
2. THE Entropy_Maximizer module SHALL assign a positive evaluation bonus proportional to the difference between the engine's legal move count and the opponent's legal move count.
3. THE Entropy_Maximizer module SHALL reside in its own Rust module file.

---

### Requirement 16: Asymmetry Addict Personality (Phase 2)

**User Story:** As a chess engine designer, I want an Asymmetry Addict personality trait, so that the engine penalizes symmetric pawn structures and rewards material or positional imbalances.

#### Acceptance Criteria

1. THE Asymmetry_Addict module SHALL implement the PersonalityEval trait.
2. THE Asymmetry_Addict module SHALL assign a negative evaluation penalty proportional to the degree of pawn structure symmetry (measured by comparing pawn file occupancy between sides).
3. THE Asymmetry_Addict module SHALL assign a positive evaluation bonus when material is imbalanced (e.g., bishop pair vs knight pair, rook vs two minor pieces).
4. THE Asymmetry_Addict module SHALL reside in its own Rust module file.

---

### Requirement 17: Momentum Tracker Personality (Phase 2)

**User Story:** As a chess engine designer, I want a Momentum Tracker personality trait, so that the engine adjusts aggression based on the evaluation trend.

#### Acceptance Criteria

1. THE Momentum_Tracker module SHALL implement the PersonalityEval trait.
2. THE Momentum_Tracker module SHALL compute a momentum value from the evaluation trend over the last 8 positions stored in GameContext.
3. WHEN the evaluation trend is positive (improving for the engine), THE Momentum_Tracker module SHALL assign a positive bonus encouraging aggressive play.
4. WHEN the evaluation trend is negative (worsening for the engine), THE Momentum_Tracker module SHALL assign a negative penalty encouraging defensive or consolidating play.
5. THE Momentum_Tracker module SHALL reside in its own Rust module file.

---

### Requirement 18: Zugzwang Hunter Personality (Phase 2)

**User Story:** As a chess engine designer, I want a Zugzwang Hunter personality trait, so that the engine seeks positions where the opponent has few good moves, especially in endgames.

#### Acceptance Criteria

1. THE Zugzwang_Hunter module SHALL implement the PersonalityEval trait.
2. THE Zugzwang_Hunter module SHALL assign a positive evaluation bonus inversely proportional to the opponent's legal move count.
3. WHILE the Game_Phase is Endgame, THE Zugzwang_Hunter module SHALL apply an increased weight multiplier to amplify the zugzwang-seeking bonus.
4. THE Zugzwang_Hunter module SHALL reside in its own Rust module file.

---

### Requirement 19: Storyteller Narrative Module (Phase 3)

**User Story:** As a chess engine designer, I want a Storyteller module, so that the engine selects a strategic theme early in the game and evaluates moves for thematic alignment.

#### Acceptance Criteria

1. THE Storyteller module SHALL select a Theme from a predefined set (e.g., kingside attack, central domination, queenside expansion, piece sacrifice preparation) between moves 3 and 5 based on the current position.
2. WHEN a Theme is active, THE Storyteller module SHALL assign a positive evaluation bonus to moves that advance the selected Theme.
3. WHEN a Theme is active, THE Storyteller module SHALL assign a negative evaluation penalty to moves that contradict the selected Theme.
4. THE Storyteller module SHALL reside in the `narrative` module directory as a submodule.

---

### Requirement 20: Time Traveler Narrative Module (Phase 3)

**User Story:** As a chess engine designer, I want a Time Traveler module, so that the engine grants extra search depth to moves aligned with the Storyteller's theme.

#### Acceptance Criteria

1. WHEN a move is aligned with the active Theme (as determined by the Storyteller), THE Time_Traveler module SHALL extend the search depth for that move by a configurable number of plies (default 1).
2. THE Time_Traveler module SHALL limit the total number of depth extensions per search to prevent search explosion.
3. THE Time_Traveler module SHALL reside in the `narrative` module directory as a submodule.

---

### Requirement 21: Mirage Narrative Module (Phase 3)

**User Story:** As a chess engine designer, I want a Mirage module, so that the engine rewards moves that conceal its strategic intention from the opponent.

#### Acceptance Criteria

1. THE Mirage module SHALL assign a positive evaluation bonus to moves that advance the active Theme while appearing to serve a different strategic purpose (multi-purpose moves).
2. THE Mirage module SHALL measure deception potential by counting the number of distinct strategic purposes a move serves beyond the active Theme.
3. THE Mirage module SHALL reside in the `narrative` module directory as a submodule.

---

### Requirement 22: Game Arc Phase Weighting

**User Story:** As a chess engine designer, I want game arc phase weighting, so that personality and narrative traits activate and deactivate according to the phase of the game.

#### Acceptance Criteria

1. WHILE the Game_Phase is Opening (moves 1-10), THE Engine SHALL apply increased weights to Romantic, Storyteller, and Mirage modules.
2. WHILE the Game_Phase is Early_Middlegame (moves 10-20), THE Engine SHALL apply increased weights to Chaos_Theory, Entropy_Maximizer, Mirage, and Time_Traveler modules.
3. WHILE the Game_Phase is Late_Middlegame (moves 20-30), THE Engine SHALL apply increased weights to Momentum_Tracker and Time_Traveler modules.
4. WHILE the Game_Phase is Endgame (moves 30+), THE Engine SHALL apply increased weights to Zugzwang_Hunter and Momentum_Tracker modules.
5. THE Game_Arc weight profiles SHALL be configurable via a weight table that maps Game_Phase to per-module weight multipliers.

---

### Requirement 23: Build and Binary Output

**User Story:** As a chess engine user, I want a single binary produced by `cargo build --release`, so that deployment is straightforward.

#### Acceptance Criteria

1. THE Engine SHALL compile as a single binary using `cargo build --release` with Rust 2021 edition.
2. THE Engine SHALL not depend on external crates for core engine logic (board representation, move generation, search, evaluation); external crates are permitted only for UCI input parsing.
3. THE Engine SHALL organize code into modules: `board`, `movegen`, `search`, `eval`, `uci`, `personality` (with submodules per trait), and `narrative` (with submodules `storyteller`, `time_traveler`, `mirage`).

---

### Requirement 24: Perft Testing and Correctness Verification

**User Story:** As a chess engine developer, I want perft testing support, so that I can verify move generation correctness against known results.

#### Acceptance Criteria

1. WHEN the Engine is invoked with a perft command (via CLI argument or UCI `go perft <depth>`), THE Engine SHALL output the total node count and per-move node counts for the given position and depth.
2. THE Engine SHALL include built-in perft test cases for at least 5 standard positions (initial position, Kiwipete, position 3, position 4, position 5 from the Chess Programming Wiki).
3. THE Engine SHALL match published perft node counts for all built-in test positions at depths up to 6.
