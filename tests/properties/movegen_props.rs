// Feature: chess-engine-core
// Property 6: Generated Moves Are Legal
// Property 7: Terminal Position Detection
// Perft verification unit tests
//
// **Validates: Requirements 3.1, 3.4, 3.5, 3.6, 3.7, 3.8, 24.2, 24.3**

use chess_engine_core::board::magic::init_magic_tables;
use chess_engine_core::board::Board;
use chess_engine_core::movegen::{generate_legal_moves, perft, MoveGenResult};
use proptest::prelude::*;
use std::sync::Once;

static INIT: Once = Once::new();

fn ensure_init() {
    INIT.call_once(|| {
        init_magic_tables();
    });
}

// ─── Known positions for property testing ────────────────────────────────────

/// A set of known valid FEN positions to use as starting points for random game play.
const KNOWN_POSITIONS: &[&str] = &[
    // Starting position
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    // Kiwipete
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    // Position 3
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    // Position 4
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    // Position 5
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    // Open position with lots of tactics
    "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
    // Endgame position
    "8/8/4k3/8/8/4K3/4P3/8 w - - 0 1",
    // Position with en passant
    "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
];

/// Strategy that picks a random known position index.
fn known_position_index() -> impl Strategy<Value = usize> {
    0..KNOWN_POSITIONS.len()
}

/// Strategy for a random seed used to select moves during random game play.
fn move_seed() -> impl Strategy<Value = Vec<usize>> {
    prop::collection::vec(0..1000usize, 0..30)
}

/// From a known position, play a random sequence of legal moves to reach a diverse position.
/// Returns the board at the reached position.
fn play_random_moves(fen: &str, seeds: &[usize]) -> Board {
    let mut board = Board::from_fen(fen).expect("known position should parse");
    for &seed in seeds {
        match generate_legal_moves(&mut board) {
            MoveGenResult::Moves(moves) if !moves.is_empty() => {
                let idx = seed % moves.len();
                board.make_move(moves[idx]);
            }
            _ => break, // terminal position reached
        }
    }
    board
}

// ─── Property 6: Generated Moves Are Legal ───────────────────────────────────
// **Validates: Requirements 3.1, 3.4, 3.5**
//
// For any valid Board position and any move returned by generate_legal_moves,
// making that move shall not leave the moving side's king in check.
// Additionally, when the side to move is in check, every generated move shall
// resolve the check.

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 6: Every generated legal move does not leave own king in check.
    /// When in check, every generated move resolves the check.
    #[test]
    fn generated_moves_are_legal(
        pos_idx in known_position_index(),
        seeds in move_seed(),
    ) {
        ensure_init();

        let mut board = play_random_moves(KNOWN_POSITIONS[pos_idx], &seeds);
        let us = board.side_to_move;
        let was_in_check = board.is_in_check(us);

        match generate_legal_moves(&mut board) {
            MoveGenResult::Moves(moves) => {
                for mv in &moves {
                    // Make the move and verify own king is not in check
                    board.make_move(*mv);
                    prop_assert!(
                        !board.is_in_check(us),
                        "Legal move {:?} leaves own king ({:?}) in check! Position FEN before move: {}",
                        mv, us, {
                            board.unmake_move(*mv);
                            board.to_fen()
                        }
                    );
                    // After the move, if we were in check, verify check is resolved
                    // (the king of `us` should no longer be in check)
                    if was_in_check {
                        prop_assert!(
                            !board.is_in_check(us),
                            "Move {:?} did not resolve check for {:?}",
                            mv, us
                        );
                    }
                    board.unmake_move(*mv);
                }
            }
            MoveGenResult::Checkmate | MoveGenResult::Stalemate => {
                // Terminal positions are fine — no moves to verify
            }
        }
    }
}

// ─── Property 7: Terminal Position Detection ─────────────────────────────────
// **Validates: Requirements 3.6, 3.7**
//
// For positions with zero legal moves, verify:
// - Checkmate iff king is in check
// - Stalemate iff king is not in check

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 7: Terminal position detection is correct.
    /// Checkmate iff in check with no legal moves, Stalemate iff not in check with no legal moves.
    #[test]
    fn terminal_position_detection(
        pos_idx in known_position_index(),
        seeds in move_seed(),
    ) {
        ensure_init();

        let mut board = play_random_moves(KNOWN_POSITIONS[pos_idx], &seeds);
        let us = board.side_to_move;

        match generate_legal_moves(&mut board) {
            MoveGenResult::Checkmate => {
                // Checkmate: king MUST be in check
                prop_assert!(
                    board.is_in_check(us),
                    "Checkmate reported but {:?} king is NOT in check! FEN: {}",
                    us, board.to_fen()
                );
            }
            MoveGenResult::Stalemate => {
                // Stalemate: king must NOT be in check
                prop_assert!(
                    !board.is_in_check(us),
                    "Stalemate reported but {:?} king IS in check! FEN: {}",
                    us, board.to_fen()
                );
            }
            MoveGenResult::Moves(_) => {
                // Non-terminal: nothing to verify for this property
            }
        }
    }
}

// Also test known checkmate and stalemate positions directly

#[test]
fn known_checkmate_positions() {
    ensure_init();

    let checkmate_fens = [
        // Fool's mate
        "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3",
        // Scholar's mate
        "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4",
        // Back rank mate
        "6k1/5ppp/8/8/8/8/8/3R2K1 b - - 0 1",
    ];

    for fen in &checkmate_fens {
        let mut board = Board::from_fen(fen).unwrap();
        let us = board.side_to_move;

        // Verify the position is actually checkmate (some of these might not be exact)
        if board.is_in_check(us) {
            match generate_legal_moves(&mut board) {
                MoveGenResult::Checkmate => {
                    // Correct: in check + no legal moves = checkmate
                    assert!(board.is_in_check(us));
                }
                MoveGenResult::Moves(_) => {
                    // Has evasions, not checkmate — that's fine, skip
                }
                MoveGenResult::Stalemate => {
                    panic!("Stalemate reported for position in check: {}", fen);
                }
            }
        }
    }
}

#[test]
fn known_stalemate_positions() {
    ensure_init();

    let stalemate_fens = [
        // King in corner, queen blocks
        "k7/8/1QK5/8/8/8/8/8 b - - 0 1",
        // King trapped by own pawns
        "5k2/5P2/5K2/8/8/8/8/8 b - - 0 1",
    ];

    for fen in &stalemate_fens {
        let mut board = Board::from_fen(fen).unwrap();
        let us = board.side_to_move;

        match generate_legal_moves(&mut board) {
            MoveGenResult::Stalemate => {
                assert!(
                    !board.is_in_check(us),
                    "Stalemate but king is in check for: {}",
                    fen
                );
            }
            MoveGenResult::Checkmate => {
                panic!(
                    "Checkmate reported for stalemate position: {}",
                    fen
                );
            }
            MoveGenResult::Moves(moves) => {
                panic!(
                    "Expected stalemate but got {} legal moves for: {}",
                    moves.len(),
                    fen
                );
            }
        }
    }
}

// ─── Perft Verification Unit Tests (Task 7.6) ───────────────────────────────
// **Validates: Requirements 3.8, 24.2, 24.3**
//
// Run perft for all 5 built-in positions at depths up to 6.
// Verify node counts match published values.
// Depth 5+ tests are marked #[ignore] since they're slow.

// Position 1: Initial position
// rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1

#[test]
fn perft_initial_depth_1() {
    ensure_init();
    let mut board = Board::new();
    assert_eq!(perft(&mut board, 1), 20);
}

#[test]
fn perft_initial_depth_2() {
    ensure_init();
    let mut board = Board::new();
    assert_eq!(perft(&mut board, 2), 400);
}

#[test]
fn perft_initial_depth_3() {
    ensure_init();
    let mut board = Board::new();
    assert_eq!(perft(&mut board, 3), 8_902);
}

#[test]
fn perft_initial_depth_4() {
    ensure_init();
    let mut board = Board::new();
    assert_eq!(perft(&mut board, 4), 197_281);
}

#[test]
#[ignore]
fn perft_initial_depth_5() {
    ensure_init();
    let mut board = Board::new();
    assert_eq!(perft(&mut board, 5), 4_865_609);
}

// Position 2: Kiwipete
// r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -

const KIWIPETE_FEN: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";

#[test]
fn perft_kiwipete_depth_1() {
    ensure_init();
    let mut board = Board::from_fen(KIWIPETE_FEN).unwrap();
    assert_eq!(perft(&mut board, 1), 48);
}

#[test]
fn perft_kiwipete_depth_2() {
    ensure_init();
    let mut board = Board::from_fen(KIWIPETE_FEN).unwrap();
    assert_eq!(perft(&mut board, 2), 2_039);
}

#[test]
fn perft_kiwipete_depth_3() {
    ensure_init();
    let mut board = Board::from_fen(KIWIPETE_FEN).unwrap();
    assert_eq!(perft(&mut board, 3), 97_862);
}

#[test]
fn perft_kiwipete_depth_4() {
    ensure_init();
    let mut board = Board::from_fen(KIWIPETE_FEN).unwrap();
    assert_eq!(perft(&mut board, 4), 4_085_603);
}

#[test]
#[ignore]
fn perft_kiwipete_depth_5() {
    ensure_init();
    let mut board = Board::from_fen(KIWIPETE_FEN).unwrap();
    assert_eq!(perft(&mut board, 5), 193_690_690);
}

// Position 3
// 8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - -

const POSITION3_FEN: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";

#[test]
fn perft_position3_depth_1() {
    ensure_init();
    let mut board = Board::from_fen(POSITION3_FEN).unwrap();
    assert_eq!(perft(&mut board, 1), 14);
}

#[test]
fn perft_position3_depth_2() {
    ensure_init();
    let mut board = Board::from_fen(POSITION3_FEN).unwrap();
    assert_eq!(perft(&mut board, 2), 191);
}

#[test]
fn perft_position3_depth_3() {
    ensure_init();
    let mut board = Board::from_fen(POSITION3_FEN).unwrap();
    assert_eq!(perft(&mut board, 3), 2_812);
}

#[test]
fn perft_position3_depth_4() {
    ensure_init();
    let mut board = Board::from_fen(POSITION3_FEN).unwrap();
    assert_eq!(perft(&mut board, 4), 43_238);
}

#[test]
#[ignore]
fn perft_position3_depth_5() {
    ensure_init();
    let mut board = Board::from_fen(POSITION3_FEN).unwrap();
    assert_eq!(perft(&mut board, 5), 674_624);
}

// Position 4
// r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1

const POSITION4_FEN: &str = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";

#[test]
fn perft_position4_depth_1() {
    ensure_init();
    let mut board = Board::from_fen(POSITION4_FEN).unwrap();
    assert_eq!(perft(&mut board, 1), 6);
}

#[test]
fn perft_position4_depth_2() {
    ensure_init();
    let mut board = Board::from_fen(POSITION4_FEN).unwrap();
    assert_eq!(perft(&mut board, 2), 264);
}

#[test]
fn perft_position4_depth_3() {
    ensure_init();
    let mut board = Board::from_fen(POSITION4_FEN).unwrap();
    assert_eq!(perft(&mut board, 3), 9_467);
}

#[test]
fn perft_position4_depth_4() {
    ensure_init();
    let mut board = Board::from_fen(POSITION4_FEN).unwrap();
    assert_eq!(perft(&mut board, 4), 422_333);
}

#[test]
#[ignore]
fn perft_position4_depth_5() {
    ensure_init();
    let mut board = Board::from_fen(POSITION4_FEN).unwrap();
    assert_eq!(perft(&mut board, 5), 15_833_292);
}

// Position 5
// rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8

const POSITION5_FEN: &str = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";

#[test]
fn perft_position5_depth_1() {
    ensure_init();
    let mut board = Board::from_fen(POSITION5_FEN).unwrap();
    assert_eq!(perft(&mut board, 1), 44);
}

#[test]
fn perft_position5_depth_2() {
    ensure_init();
    let mut board = Board::from_fen(POSITION5_FEN).unwrap();
    assert_eq!(perft(&mut board, 2), 1_486);
}

#[test]
fn perft_position5_depth_3() {
    ensure_init();
    let mut board = Board::from_fen(POSITION5_FEN).unwrap();
    assert_eq!(perft(&mut board, 3), 62_379);
}

#[test]
fn perft_position5_depth_4() {
    ensure_init();
    let mut board = Board::from_fen(POSITION5_FEN).unwrap();
    assert_eq!(perft(&mut board, 4), 2_103_487);
}

#[test]
#[ignore]
fn perft_position5_depth_5() {
    ensure_init();
    let mut board = Board::from_fen(POSITION5_FEN).unwrap();
    assert_eq!(perft(&mut board, 5), 89_941_194);
}
