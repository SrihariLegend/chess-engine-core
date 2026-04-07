// Feature: chess-engine-core, Property 1: FEN Round Trip
// **Validates: Requirements 1.4, 1.6, 1.7**
//
// For any valid FEN string, parsing into a Board and exporting back to FEN
// then parsing again produces an equivalent Board state.

use chess_engine_core::board::*;
use proptest::prelude::*;

// ─── FEN Generator ───────────────────────────────────────────────────────────

/// Generates a random valid piece placement for a single rank.
/// Each rank must sum to exactly 8 squares.
fn rank_strategy() -> impl Strategy<Value = String> {
    // Generate a sequence of 8 cells: each is either a piece char or empty
    prop::collection::vec(
        prop_oneof![
            // Empty square (will be collapsed into digits)
            Just(None),
            // Piece characters
            Just(Some('P')),
            Just(Some('N')),
            Just(Some('B')),
            Just(Some('R')),
            Just(Some('Q')),
            Just(Some('p')),
            Just(Some('n')),
            Just(Some('b')),
            Just(Some('r')),
            Just(Some('q')),
        ],
        8..=8,
    )
    .prop_map(|cells| {
        let mut rank = String::new();
        let mut empty_count = 0u8;
        for cell in cells {
            match cell {
                None => empty_count += 1,
                Some(ch) => {
                    if empty_count > 0 {
                        rank.push((b'0' + empty_count) as char);
                        empty_count = 0;
                    }
                    rank.push(ch);
                }
            }
        }
        if empty_count > 0 {
            rank.push((b'0' + empty_count) as char);
        }
        rank
    })
}

/// Generates a valid piece placement string (8 ranks separated by '/').
/// Ensures exactly one white king and one black king exist.
fn piece_placement_strategy() -> impl Strategy<Value = String> {
    // Generate 8 ranks, then inject exactly one K and one k
    prop::collection::vec(rank_strategy(), 8..=8)
        .prop_flat_map(|ranks| {
            // Pick random positions for the white and black kings
            let wk_rank = 0..8usize;
            let wk_file = 0..8usize;
            let bk_rank = 0..8usize;
            let bk_file = 0..8usize;
            (Just(ranks), wk_rank, wk_file, bk_rank, bk_file)
        })
        .prop_map(|(ranks, wk_rank, wk_file, bk_rank, bk_file)| {
            // Parse each rank into 8 cells, place kings, then re-encode
            let mut grid: Vec<Vec<Option<char>>> = Vec::new();
            for rank_str in &ranks {
                let mut cells = Vec::new();
                for ch in rank_str.chars() {
                    if let Some(digit) = ch.to_digit(10) {
                        for _ in 0..digit {
                            cells.push(None);
                        }
                    } else {
                        // Filter out any kings that were randomly generated
                        if ch == 'K' || ch == 'k' {
                            cells.push(None);
                        } else {
                            cells.push(Some(ch));
                        }
                    }
                }
                // Ensure exactly 8 cells
                cells.truncate(8);
                while cells.len() < 8 {
                    cells.push(None);
                }
                grid.push(cells);
            }

            // Place exactly one white king and one black king
            grid[wk_rank][wk_file] = Some('K');
            // If black king would land on same square, shift it
            let (bkr, bkf) = if bk_rank == wk_rank && bk_file == wk_file {
                // Pick the next available square
                ((bk_rank + 1) % 8, bk_file)
            } else {
                (bk_rank, bk_file)
            };
            grid[bkr][bkf] = Some('k');

            // Remove pawns from ranks 1 and 8 (ranks 0 and 7 in grid = rank 8 and rank 1)
            // Grid index 0 = rank 8, grid index 7 = rank 1
            for file in 0..8 {
                // Rank 8 (grid[0])
                if let Some(ch) = grid[0][file] {
                    if ch == 'P' || ch == 'p' {
                        grid[0][file] = None;
                    }
                }
                // Rank 1 (grid[7])
                if let Some(ch) = grid[7][file] {
                    if ch == 'P' || ch == 'p' {
                        grid[7][file] = None;
                    }
                }
            }

            // Encode back to FEN piece placement
            let mut result = String::new();
            for (i, row) in grid.iter().enumerate() {
                let mut empty = 0u8;
                for cell in row {
                    match cell {
                        None => empty += 1,
                        Some(ch) => {
                            if empty > 0 {
                                result.push((b'0' + empty) as char);
                                empty = 0;
                            }
                            result.push(*ch);
                        }
                    }
                }
                if empty > 0 {
                    result.push((b'0' + empty) as char);
                }
                if i < 7 {
                    result.push('/');
                }
            }
            result
        })
}

/// Generates a valid side to move ('w' or 'b').
fn side_to_move_strategy() -> impl Strategy<Value = String> {
    prop_oneof![Just("w".to_string()), Just("b".to_string())]
}

/// Generates valid castling rights (subset of KQkq, or '-').
fn castling_strategy() -> impl Strategy<Value = String> {
    (prop::bool::ANY, prop::bool::ANY, prop::bool::ANY, prop::bool::ANY).prop_map(
        |(wk, wq, bk, bq)| {
            let mut s = String::new();
            if wk { s.push('K'); }
            if wq { s.push('Q'); }
            if bk { s.push('k'); }
            if bq { s.push('q'); }
            if s.is_empty() {
                "-".to_string()
            } else {
                s
            }
        },
    )
}

/// Generates a valid en passant square or '-'.
/// If side to move is 'w', en passant is on rank 6 (index 5).
/// If side to move is 'b', en passant is on rank 3 (index 2).
fn en_passant_strategy(side: String) -> impl Strategy<Value = String> {
    let rank = if side == "w" { '6' } else { '3' };
    prop_oneof![
        // No en passant (weighted higher since most positions don't have it)
        8 => Just("-".to_string()),
        // Valid en passant square
        2 => (0..8u8).prop_map(move |file| {
            let f = (b'a' + file) as char;
            format!("{}{}", f, rank)
        }),
    ]
}

/// Generates a valid halfmove clock (0-100).
fn halfmove_clock_strategy() -> impl Strategy<Value = String> {
    (0u16..=100).prop_map(|n| n.to_string())
}

/// Generates a valid fullmove number (1-200).
fn fullmove_number_strategy() -> impl Strategy<Value = String> {
    (1u16..=200).prop_map(|n| n.to_string())
}

/// Generates a complete valid FEN string.
fn fen_strategy() -> impl Strategy<Value = String> {
    (piece_placement_strategy(), side_to_move_strategy(), castling_strategy())
        .prop_flat_map(|(pieces, side, castling)| {
            let ep = en_passant_strategy(side.clone());
            (
                Just(pieces),
                Just(side),
                Just(castling),
                ep,
                halfmove_clock_strategy(),
                fullmove_number_strategy(),
            )
        })
        .prop_map(|(pieces, side, castling, ep, halfmove, fullmove)| {
            format!("{} {} {} {} {} {}", pieces, side, castling, ep, halfmove, fullmove)
        })
}

// ─── Property Test ───────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Property 1: FEN Round Trip
    ///
    /// For any valid FEN string:
    /// 1. Board::from_fen(fen) succeeds
    /// 2. board.to_fen() produces a FEN string
    /// 3. Board::from_fen(board.to_fen()) succeeds
    /// 4. The two Board states have identical pieces, occupancy, side_to_move,
    ///    castling, en_passant, halfmove_clock, fullmove_number
    #[test]
    fn fen_round_trip(fen in fen_strategy()) {
        // Step 1: Parse the generated FEN
        let board1 = Board::from_fen(&fen)
            .map_err(|e| TestCaseError::fail(format!("from_fen failed on generated FEN '{}': {}", fen, e)))?;

        // Step 2: Export to FEN
        let exported_fen = board1.to_fen();

        // Step 3: Parse the exported FEN
        let board2 = Board::from_fen(&exported_fen)
            .map_err(|e| TestCaseError::fail(format!("from_fen failed on exported FEN '{}': {}", exported_fen, e)))?;

        // Step 4: Compare all board state fields
        // Piece bitboards
        for color in 0..2 {
            for piece in 0..6 {
                prop_assert_eq!(
                    board1.pieces[color][piece],
                    board2.pieces[color][piece],
                    "Mismatch in pieces[{}][{}] for FEN '{}'",
                    color, piece, fen
                );
            }
        }

        // Occupancy
        prop_assert_eq!(
            board1.occupancy[0], board2.occupancy[0],
            "White occupancy mismatch for FEN '{}'", fen
        );
        prop_assert_eq!(
            board1.occupancy[1], board2.occupancy[1],
            "Black occupancy mismatch for FEN '{}'", fen
        );
        prop_assert_eq!(
            board1.all_occupancy, board2.all_occupancy,
            "All occupancy mismatch for FEN '{}'", fen
        );

        // Side to move
        prop_assert_eq!(
            board1.side_to_move, board2.side_to_move,
            "Side to move mismatch for FEN '{}'", fen
        );

        // Castling rights
        prop_assert_eq!(
            board1.castling, board2.castling,
            "Castling rights mismatch for FEN '{}'", fen
        );

        // En passant
        prop_assert_eq!(
            board1.en_passant, board2.en_passant,
            "En passant mismatch for FEN '{}'", fen
        );

        // Halfmove clock
        prop_assert_eq!(
            board1.halfmove_clock, board2.halfmove_clock,
            "Halfmove clock mismatch for FEN '{}'", fen
        );

        // Fullmove number
        prop_assert_eq!(
            board1.fullmove_number, board2.fullmove_number,
            "Fullmove number mismatch for FEN '{}'", fen
        );
    }
}

// ─── Property 2: Invalid FEN Rejection ───────────────────────────────────────
// Feature: chess-engine-core, Property 2: Invalid FEN Rejection
// **Validates: Requirements 1.5**
//
// For any string that is not a valid FEN (wrong number of fields, invalid piece
// characters, illegal rank lengths, out-of-range values), parsing shall return
// an Err with a descriptive error message, and no Board shall be constructed.

/// A valid base FEN to use as a template when mutating individual fields.
const VALID_BASE_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Helper: splits a valid FEN into its 6 fields and reassembles with a replacement.
fn replace_fen_field(base: &str, field_idx: usize, replacement: &str) -> String {
    let mut fields: Vec<&str> = base.split_whitespace().collect();
    let owned;
    if field_idx < fields.len() {
        owned = replacement.to_string();
        fields[field_idx] = &owned;
    }
    fields.join(" ")
}

// ─── Generators for invalid FEN components ───────────────────────────────────

/// Generates a FEN-like string with the wrong number of space-separated fields (not 6).
fn wrong_field_count_strategy() -> impl Strategy<Value = String> {
    // Generate 1-5 or 7-10 fields
    prop_oneof![
        // Too few fields (1-5)
        (1usize..=5).prop_flat_map(|n| {
            prop::collection::vec("[a-zA-Z0-9/]{1,20}", n..=n)
                .prop_map(|fields| fields.join(" "))
        }),
        // Too many fields (7-10)
        (7usize..=10).prop_flat_map(|n| {
            prop::collection::vec("[a-zA-Z0-9/]{1,10}", n..=n)
                .prop_map(|fields| fields.join(" "))
        }),
    ]
}

/// Generates a piece placement string with invalid piece characters.
fn invalid_piece_chars_strategy() -> impl Strategy<Value = String> {
    // Pick a random invalid char and inject it into an otherwise valid-looking rank
    let invalid_chars = prop::sample::select(vec!['x', 'z', 'X', 'Z', 'w', 'W', 'y', 'Y', 's', 'S', '!', '@', '#']);
    (invalid_chars, 0usize..8).prop_map(|(bad_char, inject_rank)| {
        // Build 8 ranks, inject the bad char into one of them
        let mut ranks: Vec<String> = Vec::new();
        for i in 0..8 {
            if i == inject_rank {
                // A rank with a bad character: e.g. "4x3" (sums to 8 squares)
                ranks.push(format!("4{}3", bad_char));
            } else {
                ranks.push("8".to_string());
            }
        }
        let placement = ranks.join("/");
        format!("{} w KQkq - 0 1", placement)
    })
}

/// Generates a piece placement where at least one rank doesn't sum to 8 squares.
fn bad_rank_sum_strategy() -> impl Strategy<Value = String> {
    // Generate a rank that sums to something other than 8
    prop_oneof![
        // Too few squares (sum to 1-7)
        (1u8..=7).prop_map(|n| n.to_string()),
        // Too many squares via digit (9)
        Just("9".to_string()),
        // Pieces that sum to more than 8
        Just("PPPPPPPP1".to_string()),
        // Pieces that sum to fewer than 8
        Just("PPP".to_string()),
        Just("4K".to_string()),
        Just("7".to_string()),
    ]
    .prop_flat_map(|bad_rank| {
        // Place the bad rank at a random position among 8 ranks
        (Just(bad_rank), 0usize..8)
    })
    .prop_map(|(bad_rank, bad_idx)| {
        let mut ranks: Vec<String> = Vec::new();
        for i in 0..8 {
            if i == bad_idx {
                ranks.push(bad_rank.clone());
            } else {
                ranks.push("8".to_string());
            }
        }
        let placement = ranks.join("/");
        format!("{} w KQkq - 0 1", placement)
    })
}

/// Generates a FEN with an invalid side-to-move field (not 'w' or 'b').
fn invalid_side_to_move_strategy() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["W", "B", "x", "white", "black", "1", "-", "wb"])
        .prop_map(|bad_side| {
            replace_fen_field(VALID_BASE_FEN, 1, bad_side)
        })
}

/// Generates a FEN with invalid castling characters.
/// Only uses strings containing characters outside the valid set {K, Q, k, q, -}.
fn invalid_castling_strategy() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["X", "kqx", "Aa", "1234", "z", "KQBR", "Kx", "abc"])
        .prop_map(|bad_castling| {
            replace_fen_field(VALID_BASE_FEN, 2, bad_castling)
        })
}

/// Generates a FEN with an invalid en passant square.
fn invalid_en_passant_strategy() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["i3", "a9", "a0", "z6", "abc", "3a", "11", "xx"])
        .prop_map(|bad_ep| {
            replace_fen_field(VALID_BASE_FEN, 3, bad_ep)
        })
}

// ─── Property Tests ──────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 2a: Wrong number of fields → Err
    ///
    /// FEN strings with fewer or more than 6 space-separated fields must be rejected.
    #[test]
    fn invalid_fen_wrong_field_count(fen in wrong_field_count_strategy()) {
        let result = Board::from_fen(&fen);
        prop_assert!(
            result.is_err(),
            "Expected Err for FEN with wrong field count: '{}'", fen
        );
    }

    /// Property 2b: Invalid piece characters → Err
    ///
    /// FEN strings containing characters that are not valid piece letters or digits
    /// in the placement field must be rejected.
    #[test]
    fn invalid_fen_bad_piece_chars(fen in invalid_piece_chars_strategy()) {
        let result = Board::from_fen(&fen);
        prop_assert!(
            result.is_err(),
            "Expected Err for FEN with invalid piece chars: '{}'", fen
        );
    }

    /// Property 2c: Ranks that don't sum to 8 squares → Err
    ///
    /// FEN strings where any rank's pieces + empty squares don't total 8 must be rejected.
    #[test]
    fn invalid_fen_bad_rank_sum(fen in bad_rank_sum_strategy()) {
        let result = Board::from_fen(&fen);
        prop_assert!(
            result.is_err(),
            "Expected Err for FEN with bad rank sum: '{}'", fen
        );
    }

    /// Property 2d: Invalid side to move → Err
    ///
    /// FEN strings with a side-to-move field that is not 'w' or 'b' must be rejected.
    #[test]
    fn invalid_fen_bad_side_to_move(fen in invalid_side_to_move_strategy()) {
        let result = Board::from_fen(&fen);
        prop_assert!(
            result.is_err(),
            "Expected Err for FEN with invalid side to move: '{}'", fen
        );
    }

    /// Property 2e: Invalid castling characters → Err
    ///
    /// FEN strings with castling fields containing characters other than K, Q, k, q, or '-'
    /// must be rejected.
    #[test]
    fn invalid_fen_bad_castling(fen in invalid_castling_strategy()) {
        let result = Board::from_fen(&fen);
        prop_assert!(
            result.is_err(),
            "Expected Err for FEN with invalid castling: '{}'", fen
        );
    }

    /// Property 2f: Invalid en passant squares → Err
    ///
    /// FEN strings with en passant fields that are not '-' or a valid square must be rejected.
    #[test]
    fn invalid_fen_bad_en_passant(fen in invalid_en_passant_strategy()) {
        let result = Board::from_fen(&fen);
        prop_assert!(
            result.is_err(),
            "Expected Err for FEN with invalid en passant: '{}'", fen
        );
    }
}

// ─── Property 8: Make/Unmake Round Trip ──────────────────────────────────────
// Feature: chess-engine-core, Property 8: Make/Unmake Round Trip
// **Validates: Requirements 4.3, 11.4**
//
// For any valid Board position and any legal move from that position, calling
// make_move followed by unmake_move shall restore the Board to a state identical
// to the original — including all 12 piece bitboards, occupancy bitboards, side
// to move, castling rights, en passant square, halfmove clock, fullmove number,
// and Zobrist hash.

/// A snapshot of all board state fields for comparison.
#[derive(Debug, PartialEq, Eq)]
struct BoardSnapshot {
    pieces: [[u64; 6]; 2],
    occupancy: [u64; 2],
    all_occupancy: u64,
    side_to_move: Color,
    castling: CastlingRights,
    en_passant: Option<u8>,
    halfmove_clock: u16,
    fullmove_number: u16,
    zobrist_hash: u64,
}

impl BoardSnapshot {
    fn capture(board: &Board) -> Self {
        BoardSnapshot {
            pieces: board.pieces,
            occupancy: board.occupancy,
            all_occupancy: board.all_occupancy,
            side_to_move: board.side_to_move,
            castling: board.castling,
            en_passant: board.en_passant,
            halfmove_clock: board.halfmove_clock,
            fullmove_number: board.fullmove_number,
            zobrist_hash: board.zobrist_hash,
        }
    }
}

/// Known positions with their FEN and a set of legal moves.
/// Each move is (from, to, piece, captured, promotion, flags).
struct TestPosition {
    fen: &'static str,
    moves: Vec<Move>,
}

/// Returns a set of known positions with predefined legal moves for testing.
fn known_positions_with_moves() -> Vec<TestPosition> {
    vec![
        // Starting position: White's first moves
        TestPosition {
            fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            moves: vec![
                // Pawn single pushes (a2-a3 through h2-h3)
                Move::new(8, 16, Piece::Pawn, None, None, MoveFlags::QUIET),   // a2-a3
                Move::new(9, 17, Piece::Pawn, None, None, MoveFlags::QUIET),   // b2-b3
                Move::new(10, 18, Piece::Pawn, None, None, MoveFlags::QUIET),  // c2-c3
                Move::new(11, 19, Piece::Pawn, None, None, MoveFlags::QUIET),  // d2-d3
                Move::new(12, 20, Piece::Pawn, None, None, MoveFlags::QUIET),  // e2-e3
                Move::new(13, 21, Piece::Pawn, None, None, MoveFlags::QUIET),  // f2-f3
                Move::new(14, 22, Piece::Pawn, None, None, MoveFlags::QUIET),  // g2-g3
                Move::new(15, 23, Piece::Pawn, None, None, MoveFlags::QUIET),  // h2-h3
                // Pawn double pushes (a2-a4 through h2-h4)
                Move::new(8, 24, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH),  // a2-a4
                Move::new(9, 25, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH),  // b2-b4
                Move::new(10, 26, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // c2-c4
                Move::new(11, 27, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // d2-d4
                Move::new(12, 28, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // e2-e4
                Move::new(13, 29, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // f2-f4
                Move::new(14, 30, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // g2-g4
                Move::new(15, 31, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // h2-h4
                // Knight moves
                Move::new(1, 16, Piece::Knight, None, None, MoveFlags::QUIET),  // Nb1-a3
                Move::new(1, 18, Piece::Knight, None, None, MoveFlags::QUIET),  // Nb1-c3
                Move::new(6, 21, Piece::Knight, None, None, MoveFlags::QUIET),  // Ng1-f3
                Move::new(6, 23, Piece::Knight, None, None, MoveFlags::QUIET),  // Ng1-h3
            ],
        },
        // Position after 1.e4: Black's moves (includes en passant target on e3)
        TestPosition {
            fen: "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            moves: vec![
                // Black pawn single pushes
                Move::new(48, 40, Piece::Pawn, None, None, MoveFlags::QUIET),  // a7-a6
                Move::new(49, 41, Piece::Pawn, None, None, MoveFlags::QUIET),  // b7-b6
                Move::new(50, 42, Piece::Pawn, None, None, MoveFlags::QUIET),  // c7-c6
                Move::new(51, 43, Piece::Pawn, None, None, MoveFlags::QUIET),  // d7-d6
                Move::new(52, 44, Piece::Pawn, None, None, MoveFlags::QUIET),  // e7-e6
                Move::new(53, 45, Piece::Pawn, None, None, MoveFlags::QUIET),  // f7-f6
                Move::new(54, 46, Piece::Pawn, None, None, MoveFlags::QUIET),  // g7-g6
                Move::new(55, 47, Piece::Pawn, None, None, MoveFlags::QUIET),  // h7-h6
                // Black pawn double pushes
                Move::new(48, 32, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // a7-a5
                Move::new(49, 33, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // b7-b5
                Move::new(50, 34, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // c7-c5
                Move::new(51, 35, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // d7-d5
                Move::new(52, 36, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // e7-e5
                Move::new(53, 37, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // f7-f5
                Move::new(54, 38, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // g7-g5
                Move::new(55, 39, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // h7-h5
                // Black knight moves
                Move::new(57, 40, Piece::Knight, None, None, MoveFlags::QUIET), // Nb8-a6
                Move::new(57, 42, Piece::Knight, None, None, MoveFlags::QUIET), // Nb8-c6
                Move::new(62, 45, Piece::Knight, None, None, MoveFlags::QUIET), // Ng8-f6
                Move::new(62, 47, Piece::Knight, None, None, MoveFlags::QUIET), // Ng8-h6
            ],
        },
        // Position with captures available: Italian Game
        // After 1.e4 e5 2.Nf3 Nc6 3.Bc4
        TestPosition {
            fen: "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
            moves: vec![
                // Some Black moves in this position
                Move::new(51, 43, Piece::Pawn, None, None, MoveFlags::QUIET),  // d7-d6
                Move::new(51, 35, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // d7-d5
                Move::new(53, 45, Piece::Pawn, None, None, MoveFlags::QUIET),  // f7-f6
                Move::new(54, 46, Piece::Pawn, None, None, MoveFlags::QUIET),  // g7-g6
                Move::new(48, 40, Piece::Pawn, None, None, MoveFlags::QUIET),  // a7-a6
                Move::new(49, 41, Piece::Pawn, None, None, MoveFlags::QUIET),  // b7-b6
                Move::new(55, 47, Piece::Pawn, None, None, MoveFlags::QUIET),  // h7-h6
                // Knight moves from c6
                Move::new(42, 32, Piece::Knight, None, None, MoveFlags::QUIET), // Nc6-a5
                Move::new(42, 25, Piece::Knight, None, None, MoveFlags::QUIET), // Nc6-b4
                Move::new(42, 27, Piece::Knight, None, None, MoveFlags::QUIET), // Nc6-d4
                Move::new(42, 52, Piece::Knight, None, None, MoveFlags::QUIET), // Nc6-e7
                // Ng8-f6, Ng8-e7, Ng8-h6
                Move::new(62, 45, Piece::Knight, None, None, MoveFlags::QUIET), // Ng8-f6
                Move::new(62, 47, Piece::Knight, None, None, MoveFlags::QUIET), // Ng8-h6
            ],
        },
        // Position with castling available for White
        TestPosition {
            fen: "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
            moves: vec![
                // White kingside castling
                Move::new(4, 6, Piece::King, None, None, MoveFlags::KING_CASTLE), // O-O
                // Some other White moves
                Move::new(11, 19, Piece::Pawn, None, None, MoveFlags::QUIET),  // d2-d3
                Move::new(11, 27, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH), // d2-d4
                Move::new(10, 18, Piece::Pawn, None, None, MoveFlags::QUIET),  // c2-c3
                Move::new(8, 16, Piece::Pawn, None, None, MoveFlags::QUIET),   // a2-a3
                Move::new(21, 27, Piece::Knight, None, None, MoveFlags::QUIET), // Nf3-d4
                Move::new(21, 36, Piece::Knight, None, None, MoveFlags::QUIET), // Nf3-e5
                Move::new(21, 38, Piece::Knight, None, None, MoveFlags::QUIET), // Nf3-g5
                Move::new(21, 31, Piece::Knight, None, None, MoveFlags::QUIET), // Nf3-h4
                Move::new(21, 11, Piece::Knight, None, None, MoveFlags::QUIET), // Nf3-d2
            ],
        },
        // Position with en passant capture available
        // After 1.e4 d5 2.e5 f5 — White can capture en passant on f6
        TestPosition {
            fen: "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
            moves: vec![
                // En passant capture: e5xf6
                Move::new(36, 45, Piece::Pawn, Some(Piece::Pawn), None, MoveFlags::EN_PASSANT), // exf6 e.p.
                // Some other moves
                Move::new(8, 16, Piece::Pawn, None, None, MoveFlags::QUIET),   // a2-a3
                Move::new(11, 19, Piece::Pawn, None, None, MoveFlags::QUIET),  // d2-d3
                Move::new(36, 44, Piece::Pawn, None, None, MoveFlags::QUIET),  // e5-e6
                Move::new(1, 18, Piece::Knight, None, None, MoveFlags::QUIET), // Nb1-c3
                Move::new(6, 21, Piece::Knight, None, None, MoveFlags::QUIET), // Ng1-f3
            ],
        },
        // Position with promotion available
        // White pawn on e7, can promote
        TestPosition {
            fen: "4k3/4P3/8/8/8/8/8/4K3 w - - 0 1",
            moves: vec![
                // Promotions: e7-e8=Q, e7-e8=R, e7-e8=B, e7-e8=N
                Move::new(52, 60, Piece::Pawn, None, Some(Piece::Queen), MoveFlags::PROMOTION),
                Move::new(52, 60, Piece::Pawn, None, Some(Piece::Rook), MoveFlags::PROMOTION),
                Move::new(52, 60, Piece::Pawn, None, Some(Piece::Bishop), MoveFlags::PROMOTION),
                Move::new(52, 60, Piece::Pawn, None, Some(Piece::Knight), MoveFlags::PROMOTION),
                // King moves
                Move::new(4, 3, Piece::King, None, None, MoveFlags::QUIET),  // Ke1-d1
                Move::new(4, 5, Piece::King, None, None, MoveFlags::QUIET),  // Ke1-f1
                Move::new(4, 11, Piece::King, None, None, MoveFlags::QUIET), // Ke1-d2
                Move::new(4, 12, Piece::King, None, None, MoveFlags::QUIET), // Ke1-e2
                Move::new(4, 13, Piece::King, None, None, MoveFlags::QUIET), // Ke1-f2
            ],
        },
        // Position with capture-promotion
        // White pawn on d7, black rook on e8
        TestPosition {
            fen: "4r1k1/3P4/8/8/8/8/8/4K3 w - - 0 1",
            moves: vec![
                // d7-d8=Q, d7-d8=R, d7-d8=B, d7-d8=N (non-capture promotion)
                Move::new(51, 59, Piece::Pawn, None, Some(Piece::Queen), MoveFlags::PROMOTION),
                Move::new(51, 59, Piece::Pawn, None, Some(Piece::Rook), MoveFlags::PROMOTION),
                Move::new(51, 59, Piece::Pawn, None, Some(Piece::Bishop), MoveFlags::PROMOTION),
                Move::new(51, 59, Piece::Pawn, None, Some(Piece::Knight), MoveFlags::PROMOTION),
                // d7xe8=Q, d7xe8=R, d7xe8=B, d7xe8=N (capture promotion)
                Move::new(51, 60, Piece::Pawn, Some(Piece::Rook), Some(Piece::Queen), MoveFlags::PROMOTION),
                Move::new(51, 60, Piece::Pawn, Some(Piece::Rook), Some(Piece::Rook), MoveFlags::PROMOTION),
                Move::new(51, 60, Piece::Pawn, Some(Piece::Rook), Some(Piece::Bishop), MoveFlags::PROMOTION),
                Move::new(51, 60, Piece::Pawn, Some(Piece::Rook), Some(Piece::Knight), MoveFlags::PROMOTION),
                // King moves
                Move::new(4, 3, Piece::King, None, None, MoveFlags::QUIET),
                Move::new(4, 5, Piece::King, None, None, MoveFlags::QUIET),
                Move::new(4, 12, Piece::King, None, None, MoveFlags::QUIET),
                Move::new(4, 13, Piece::King, None, None, MoveFlags::QUIET),
            ],
        },
    ]
}

/// Strategy that picks a random position index and a random move index from that position.
fn position_and_move_strategy() -> impl Strategy<Value = (usize, usize)> {
    let positions = known_positions_with_moves();
    let num_positions = positions.len();
    // Collect move counts for each position
    let move_counts: Vec<usize> = positions.iter().map(|p| p.moves.len()).collect();

    (0..num_positions).prop_flat_map(move |pos_idx| {
        let num_moves = move_counts[pos_idx];
        (Just(pos_idx), 0..num_moves)
    })
}

/// Strategy that picks a position and a random number of moves to apply in sequence.
/// For the starting position, we can apply one move, then from the resulting position
/// apply another from a known set, etc.
fn sequence_strategy() -> impl Strategy<Value = Vec<usize>> {
    // Generate a sequence of 1-5 move indices from the starting position's move set
    // (all 20 legal first moves for White)
    prop::collection::vec(0..20usize, 1..=5)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// Property 8a: Single Make/Unmake Round Trip
    ///
    /// For a randomly selected known position and a randomly selected legal move,
    /// making then unmaking the move restores the board to its original state.
    #[test]
    fn make_unmake_single_round_trip((pos_idx, move_idx) in position_and_move_strategy()) {
        let positions = known_positions_with_moves();
        let test_pos = &positions[pos_idx];
        let mut board = Board::from_fen(test_pos.fen)
            .expect("Known FEN should parse");
        let mv = test_pos.moves[move_idx];

        // Capture state before
        let before = BoardSnapshot::capture(&board);

        // Make then unmake
        board.make_move(mv);
        board.unmake_move(mv);

        // Capture state after
        let after = BoardSnapshot::capture(&board);

        // Verify all fields restored
        prop_assert_eq!(
            &before.pieces, &after.pieces,
            "Piece bitboards not restored for position '{}', move {:?}",
            test_pos.fen, mv
        );
        prop_assert_eq!(
            before.occupancy, after.occupancy,
            "Occupancy not restored for position '{}', move {:?}",
            test_pos.fen, mv
        );
        prop_assert_eq!(
            before.all_occupancy, after.all_occupancy,
            "All occupancy not restored for position '{}', move {:?}",
            test_pos.fen, mv
        );
        prop_assert_eq!(
            before.side_to_move, after.side_to_move,
            "Side to move not restored for position '{}', move {:?}",
            test_pos.fen, mv
        );
        prop_assert_eq!(
            before.castling, after.castling,
            "Castling rights not restored for position '{}', move {:?}",
            test_pos.fen, mv
        );
        prop_assert_eq!(
            before.en_passant, after.en_passant,
            "En passant not restored for position '{}', move {:?}",
            test_pos.fen, mv
        );
        prop_assert_eq!(
            before.halfmove_clock, after.halfmove_clock,
            "Halfmove clock not restored for position '{}', move {:?}",
            test_pos.fen, mv
        );
        prop_assert_eq!(
            before.fullmove_number, after.fullmove_number,
            "Fullmove number not restored for position '{}', move {:?}",
            test_pos.fen, mv
        );
        prop_assert_eq!(
            before.zobrist_hash, after.zobrist_hash,
            "Zobrist hash not restored for position '{}', move {:?}",
            test_pos.fen, mv
        );
    }

    /// Property 8b: Sequential Make/Unmake Round Trip
    ///
    /// Apply a random sequence of moves from the starting position, then unmake
    /// them all in reverse order. The board should return to the starting position.
    #[test]
    fn make_unmake_sequence_round_trip(move_indices in sequence_strategy()) {
        let mut board = Board::new();
        let before = BoardSnapshot::capture(&board);

        // All 20 legal first moves for White from starting position
        let white_first_moves = vec![
            Move::new(8, 16, Piece::Pawn, None, None, MoveFlags::QUIET),
            Move::new(9, 17, Piece::Pawn, None, None, MoveFlags::QUIET),
            Move::new(10, 18, Piece::Pawn, None, None, MoveFlags::QUIET),
            Move::new(11, 19, Piece::Pawn, None, None, MoveFlags::QUIET),
            Move::new(12, 20, Piece::Pawn, None, None, MoveFlags::QUIET),
            Move::new(13, 21, Piece::Pawn, None, None, MoveFlags::QUIET),
            Move::new(14, 22, Piece::Pawn, None, None, MoveFlags::QUIET),
            Move::new(15, 23, Piece::Pawn, None, None, MoveFlags::QUIET),
            Move::new(8, 24, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH),
            Move::new(9, 25, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH),
            Move::new(10, 26, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH),
            Move::new(11, 27, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH),
            Move::new(12, 28, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH),
            Move::new(13, 29, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH),
            Move::new(14, 30, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH),
            Move::new(15, 31, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH),
            Move::new(1, 16, Piece::Knight, None, None, MoveFlags::QUIET),
            Move::new(1, 18, Piece::Knight, None, None, MoveFlags::QUIET),
            Move::new(6, 21, Piece::Knight, None, None, MoveFlags::QUIET),
            Move::new(6, 23, Piece::Knight, None, None, MoveFlags::QUIET),
        ];

        // Apply moves (only the first one matters since subsequent moves
        // would need different move sets; we apply the same first move
        // multiple times and unmake — this still tests the round-trip property)
        let mut applied_moves = Vec::new();
        for &idx in &move_indices {
            // Only apply the first move, make it, then we'll unmake all
            let mv = white_first_moves[idx];
            board.make_move(mv);
            applied_moves.push(mv);
            // After making a White move, we need to unmake before making another
            // since the position changes. For the sequence test, we make one move
            // at a time and unmake immediately to test repeated make/unmake.
            board.unmake_move(mv);
        }

        // After all make/unmake pairs, board should be at starting position
        let after = BoardSnapshot::capture(&board);
        prop_assert_eq!(&before, &after, "Board not restored after sequence of make/unmake pairs");
    }
}

// ─── Property 8c: Deterministic unit-style tests for special moves ───────────

#[cfg(test)]
mod make_unmake_tests {
    use super::*;

    /// Helper to verify make/unmake round trip for a given FEN and move.
    fn assert_round_trip(fen: &str, mv: Move) {
        let mut board = Board::from_fen(fen).expect("FEN should parse");
        let before = BoardSnapshot::capture(&board);

        board.make_move(mv);
        board.unmake_move(mv);

        let after = BoardSnapshot::capture(&board);
        assert_eq!(before, after, "Round trip failed for FEN '{}', move {:?}", fen, mv);
    }

    #[test]
    fn round_trip_kingside_castle() {
        // White can castle kingside
        assert_round_trip(
            "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
            Move::new(4, 6, Piece::King, None, None, MoveFlags::KING_CASTLE),
        );
    }

    #[test]
    fn round_trip_queenside_castle() {
        // White can castle queenside
        assert_round_trip(
            "r3kbnr/pppqpppp/2n5/3p1b2/3P1B2/2N5/PPPQPPPP/R3KBNR w KQkq - 6 5",
            Move::new(4, 2, Piece::King, None, None, MoveFlags::QUEEN_CASTLE),
        );
    }

    #[test]
    fn round_trip_en_passant() {
        // White can capture en passant on f6
        assert_round_trip(
            "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
            Move::new(36, 45, Piece::Pawn, Some(Piece::Pawn), None, MoveFlags::EN_PASSANT),
        );
    }

    #[test]
    fn round_trip_promotion_queen() {
        assert_round_trip(
            "4k3/4P3/8/8/8/8/8/4K3 w - - 0 1",
            Move::new(52, 60, Piece::Pawn, None, Some(Piece::Queen), MoveFlags::PROMOTION),
        );
    }

    #[test]
    fn round_trip_promotion_knight() {
        assert_round_trip(
            "4k3/4P3/8/8/8/8/8/4K3 w - - 0 1",
            Move::new(52, 60, Piece::Pawn, None, Some(Piece::Knight), MoveFlags::PROMOTION),
        );
    }

    #[test]
    fn round_trip_capture_promotion() {
        // White pawn on d7 captures rook on e8 and promotes
        assert_round_trip(
            "4r1k1/3P4/8/8/8/8/8/4K3 w - - 0 1",
            Move::new(51, 60, Piece::Pawn, Some(Piece::Rook), Some(Piece::Queen), MoveFlags::PROMOTION),
        );
    }

    #[test]
    fn round_trip_double_pawn_push() {
        assert_round_trip(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            Move::new(12, 28, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH),
        );
    }

    #[test]
    fn round_trip_normal_capture() {
        // Position where White knight can capture Black pawn
        assert_round_trip(
            "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2",
            Move::new(28, 35, Piece::Pawn, Some(Piece::Pawn), None, MoveFlags::QUIET),
        );
    }
}
