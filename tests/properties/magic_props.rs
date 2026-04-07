// Feature: chess-engine-core
// Property 3: Sliding Piece Attack Correctness (Model-Based)
// Property 4: Queen Attack Composition
// Property 5: Non-Sliding Piece Attack Correctness
//
// **Validates: Requirements 2.2, 2.3, 2.4, 2.5, 2.6**

use chess_engine_core::board::magic;
use chess_engine_core::board::Color;
use proptest::prelude::*;

// ─── Reference implementations for non-sliding pieces (Property 5) ───────────

/// Reference knight attacks computed from movement rules.
fn knight_attacks_ref(sq: u8) -> u64 {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let offsets: [(i8, i8); 8] = [
        (-2, -1), (-2, 1),
        (-1, -2), (-1, 2),
        ( 1, -2), ( 1, 2),
        ( 2, -1), ( 2, 1),
    ];
    let mut attacks = 0u64;
    for (dr, df) in offsets {
        let r = rank + dr;
        let f = file + df;
        if r >= 0 && r < 8 && f >= 0 && f < 8 {
            attacks |= 1u64 << (r as u8 * 8 + f as u8);
        }
    }
    attacks
}

/// Reference king attacks computed from movement rules.
fn king_attacks_ref(sq: u8) -> u64 {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let offsets: [(i8, i8); 8] = [
        (-1, -1), (-1, 0), (-1, 1),
        ( 0, -1),          ( 0, 1),
        ( 1, -1), ( 1, 0), ( 1, 1),
    ];
    let mut attacks = 0u64;
    for (dr, df) in offsets {
        let r = rank + dr;
        let f = file + df;
        if r >= 0 && r < 8 && f >= 0 && f < 8 {
            attacks |= 1u64 << (r as u8 * 8 + f as u8);
        }
    }
    attacks
}

/// Reference pawn attacks computed from movement rules.
fn pawn_attacks_ref(sq: u8, color: Color) -> u64 {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let dr: i8 = match color {
        Color::White => 1,
        Color::Black => -1,
    };
    let mut attacks = 0u64;
    let r = rank + dr;
    if r >= 0 && r < 8 {
        if file - 1 >= 0 {
            attacks |= 1u64 << (r as u8 * 8 + (file - 1) as u8);
        }
        if file + 1 < 8 {
            attacks |= 1u64 << (r as u8 * 8 + (file + 1) as u8);
        }
    }
    attacks
}

// ─── Strategies ──────────────────────────────────────────────────────────────

/// Strategy for a valid square index (0..64).
fn square_strategy() -> impl Strategy<Value = u8> {
    0u8..64
}

/// Strategy for a random occupancy bitboard.
fn occupancy_strategy() -> impl Strategy<Value = u64> {
    any::<u64>()
}

// ─── Property 3: Sliding Piece Attack Correctness (Model-Based) ──────────────
// **Validates: Requirements 2.2, 2.3**
//
// For any (square, occupancy) pair, the magic bitboard lookup for bishop and
// rook attacks must match the reference ray-casting implementation.

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn bishop_attacks_match_reference(sq in square_strategy(), occ in occupancy_strategy()) {
        let magic_result = magic::bishop_attacks(sq, occ);
        let ref_result = magic::bishop_attacks_ref(sq, occ);
        prop_assert_eq!(
            magic_result, ref_result,
            "Bishop attacks mismatch for sq={}, occ={:#018x}: magic={:#018x}, ref={:#018x}",
            sq, occ, magic_result, ref_result
        );
    }

    #[test]
    fn rook_attacks_match_reference(sq in square_strategy(), occ in occupancy_strategy()) {
        let magic_result = magic::rook_attacks(sq, occ);
        let ref_result = magic::rook_attacks_ref(sq, occ);
        prop_assert_eq!(
            magic_result, ref_result,
            "Rook attacks mismatch for sq={}, occ={:#018x}: magic={:#018x}, ref={:#018x}",
            sq, occ, magic_result, ref_result
        );
    }
}

// ─── Property 4: Queen Attack Composition ────────────────────────────────────
// **Validates: Requirements 2.4**
//
// For any (square, occupancy), queen_attacks(sq, occ) must equal
// bishop_attacks(sq, occ) | rook_attacks(sq, occ).

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn queen_attacks_equal_bishop_or_rook(sq in square_strategy(), occ in occupancy_strategy()) {
        let queen = magic::queen_attacks(sq, occ);
        let bishop = magic::bishop_attacks(sq, occ);
        let rook = magic::rook_attacks(sq, occ);
        let expected = bishop | rook;
        prop_assert_eq!(
            queen, expected,
            "Queen attacks mismatch for sq={}, occ={:#018x}: queen={:#018x}, bishop|rook={:#018x}",
            sq, occ, queen, expected
        );
    }
}

// ─── Property 5: Non-Sliding Piece Attack Correctness ────────────────────────
// **Validates: Requirements 2.5, 2.6**
//
// For all 64 squares, the precomputed knight, king, and pawn attack tables
// must match reference implementations.

#[test]
fn knight_attacks_match_reference_all_squares() {
    for sq in 0..64u8 {
        let table_result = magic::knight_attacks(sq);
        let ref_result = knight_attacks_ref(sq);
        assert_eq!(
            table_result, ref_result,
            "Knight attacks mismatch for sq={}: table={:#018x}, ref={:#018x}",
            sq, table_result, ref_result
        );
    }
}

#[test]
fn king_attacks_match_reference_all_squares() {
    for sq in 0..64u8 {
        let table_result = magic::king_attacks(sq);
        let ref_result = king_attacks_ref(sq);
        assert_eq!(
            table_result, ref_result,
            "King attacks mismatch for sq={}: table={:#018x}, ref={:#018x}",
            sq, table_result, ref_result
        );
    }
}

#[test]
fn pawn_attacks_white_match_reference_all_squares() {
    for sq in 0..64u8 {
        let table_result = magic::pawn_attacks(sq, Color::White);
        let ref_result = pawn_attacks_ref(sq, Color::White);
        assert_eq!(
            table_result, ref_result,
            "White pawn attacks mismatch for sq={}: table={:#018x}, ref={:#018x}",
            sq, table_result, ref_result
        );
    }
}

#[test]
fn pawn_attacks_black_match_reference_all_squares() {
    for sq in 0..64u8 {
        let table_result = magic::pawn_attacks(sq, Color::Black);
        let ref_result = pawn_attacks_ref(sq, Color::Black);
        assert_eq!(
            table_result, ref_result,
            "Black pawn attacks mismatch for sq={}: table={:#018x}, ref={:#018x}",
            sq, table_result, ref_result
        );
    }
}
