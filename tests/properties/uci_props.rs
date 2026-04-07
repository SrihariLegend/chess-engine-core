// Property-based tests for UCI protocol handler
// Feature: chess-engine-core, Properties 20-21

use chess_engine_core::board::magic::init_magic_tables;
use chess_engine_core::board::Board;
use chess_engine_core::movegen::{self, MoveGenResult};
use chess_engine_core::search::{format_move, SearchParams, SearchState};
use chess_engine_core::uci::{parse_move, UciHandler};
use proptest::prelude::*;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn init() {
    init_magic_tables();
}

/// Generate a random sequence of legal moves from the starting position.
/// Returns (number_of_moves, list_of_move_strings).
fn gen_move_sequence(max_moves: usize) -> impl Strategy<Value = Vec<String>> {
    // We generate a seed and number of moves, then deterministically pick moves
    (0..max_moves, any::<u64>()).prop_map(move |(num_moves, seed)| {
        init();
        let mut board = Board::new();
        let mut move_strings = Vec::new();
        let mut rng_state = seed;

        for _ in 0..num_moves {
            let moves = match movegen::generate_legal_moves(&mut board) {
                MoveGenResult::Moves(m) if !m.is_empty() => m,
                _ => break, // Game over
            };

            // Simple PRNG to pick a move
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            rng_state ^= rng_state << 17;
            let idx = (rng_state as usize) % moves.len();
            let mv = moves[idx];

            move_strings.push(format_move(&mv));
            board.make_move(mv);
        }

        move_strings
    })
}

// ─── Property 20: UCI Position Command Correctness ───────────────────────────
// Feature: chess-engine-core, Property 20: UCI Position Command Correctness
//
// For valid FEN + move sequences, verify position command produces same Board
// as manual FEN parse + make_move.
//
// **Validates: Requirements 9.3**

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_20_uci_position_startpos_moves_correctness(
        move_strs in gen_move_sequence(20)
    ) {
        init();

        // Method 1: Use UCI handler's process_command
        let mut handler = UciHandler::new();
        let cmd = if move_strs.is_empty() {
            "position startpos".to_string()
        } else {
            format!("position startpos moves {}", move_strs.join(" "))
        };
        handler.process_command(&cmd);

        // Method 2: Manual Board::new() + make_move for each move
        let mut manual_board = Board::new();
        for ms in &move_strs {
            let mv = parse_move(&mut manual_board, ms)
                .expect(&format!("parse_move should succeed for '{}'", ms));
            manual_board.make_move(mv);
        }

        // Compare: all bitboards, side to move, castling, en passant, zobrist
        for color in 0..2 {
            for piece in 0..6 {
                prop_assert_eq!(
                    handler.board.pieces[color][piece],
                    manual_board.pieces[color][piece],
                    "Mismatch in pieces[{}][{}] after moves: {:?}",
                    color, piece, move_strs
                );
            }
        }
        prop_assert_eq!(handler.board.side_to_move, manual_board.side_to_move);
        prop_assert_eq!(handler.board.castling, manual_board.castling);
        prop_assert_eq!(handler.board.en_passant, manual_board.en_passant);
        prop_assert_eq!(handler.board.halfmove_clock, manual_board.halfmove_clock);
        prop_assert_eq!(handler.board.fullmove_number, manual_board.fullmove_number);
        prop_assert_eq!(handler.board.zobrist_hash, manual_board.zobrist_hash);
    }
}

// ─── Property 21: Bestmove Output Format ─────────────────────────────────────
// Feature: chess-engine-core, Property 21: Bestmove Output Format
//
// Verify bestmove output matches `bestmove [a-h][1-8][a-h][1-8][qrbn]?`
//
// **Validates: Requirements 9.6**

/// Test positions that should produce a bestmove.
const TEST_POSITIONS: &[&str] = &[
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    // A position where promotion is likely the best move
    "8/P3k3/8/8/8/8/8/4K3 w - - 0 1",
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(TEST_POSITIONS.len() as u32))]

    #[test]
    fn prop_21_bestmove_output_format(idx in 0..TEST_POSITIONS.len()) {
        init();

        let fen = TEST_POSITIONS[idx];
        let mut board = Board::from_fen(fen).unwrap();
        let mut search_state = SearchState::new(1);
        let mut params = SearchParams::new();
        params.max_depth = Some(2);

        let best = search_state.search(&mut board, params);
        prop_assert!(best.is_some(), "Search should find a move for position: {}", fen);

        let move_str = format_move(&best.unwrap());

        // Verify format: [a-h][1-8][a-h][1-8][qrbn]?
        let bytes = move_str.as_bytes();
        prop_assert!(
            bytes.len() == 4 || bytes.len() == 5,
            "Move string should be 4 or 5 chars, got '{}' (len {})",
            move_str, bytes.len()
        );
        prop_assert!(
            bytes[0] >= b'a' && bytes[0] <= b'h',
            "From file should be a-h, got '{}'", move_str
        );
        prop_assert!(
            bytes[1] >= b'1' && bytes[1] <= b'8',
            "From rank should be 1-8, got '{}'", move_str
        );
        prop_assert!(
            bytes[2] >= b'a' && bytes[2] <= b'h',
            "To file should be a-h, got '{}'", move_str
        );
        prop_assert!(
            bytes[3] >= b'1' && bytes[3] <= b'8',
            "To rank should be 1-8, got '{}'", move_str
        );
        if bytes.len() == 5 {
            prop_assert!(
                bytes[4] == b'q' || bytes[4] == b'r' || bytes[4] == b'b' || bytes[4] == b'n',
                "Promotion suffix should be q/r/b/n, got '{}'", move_str
            );
        }
    }
}
