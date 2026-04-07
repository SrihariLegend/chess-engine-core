// Static evaluation, tapered eval, piece-square tables

use crate::board::{Board, Color, Piece};
use crate::board::magic;

// ─── Piece Values (centipawns) ───────────────────────────────────────────────

const PAWN_VALUE: i32 = 100;
const KNIGHT_VALUE: i32 = 320;
const BISHOP_VALUE: i32 = 330;
const ROOK_VALUE: i32 = 500;
const QUEEN_VALUE: i32 = 900;

pub const MATE_SCORE: i32 = 30_000;

/// Returns the material value of a piece in centipawns.
pub fn piece_value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => PAWN_VALUE,
        Piece::Knight => KNIGHT_VALUE,
        Piece::Bishop => BISHOP_VALUE,
        Piece::Rook => ROOK_VALUE,
        Piece::Queen => QUEEN_VALUE,
        Piece::King => 0,
    }
}

// ─── Piece-Square Tables ─────────────────────────────────────────────────────
//
// Tables are from White's perspective, with a1=index 0, h8=index 63.
// For Black, we mirror the square: mirror(sq) = sq ^ 56 (flip rank).
//
// Based on the Simplified Evaluation Function from the Chess Programming Wiki.

/// Middlegame piece-square tables: [piece_index][square]
#[rustfmt::skip]
static PST_MG: [[i32; 64]; 6] = [
    // Pawn (index 0)
    [
         0,  0,  0,  0,  0,  0,  0,  0,
        50, 50, 50, 50, 50, 50, 50, 50,
        10, 10, 20, 30, 30, 20, 10, 10,
         5,  5, 10, 25, 25, 10,  5,  5,
         0,  0,  0, 20, 20,  0,  0,  0,
         5, -5,-10,  0,  0,-10, -5,  5,
         5, 10, 10,-20,-20, 10, 10,  5,
         0,  0,  0,  0,  0,  0,  0,  0,
    ],
    // Knight (index 1)
    [
       -50,-40,-30,-30,-30,-30,-40,-50,
       -40,-20,  0,  0,  0,  0,-20,-40,
       -30,  0, 10, 15, 15, 10,  0,-30,
       -30,  5, 15, 20, 20, 15,  5,-30,
       -30,  0, 15, 20, 20, 15,  0,-30,
       -30,  5, 10, 15, 15, 10,  5,-30,
       -40,-20,  0,  5,  5,  0,-20,-40,
       -50,-40,-30,-30,-30,-30,-40,-50,
    ],
    // Bishop (index 2)
    [
       -20,-10,-10,-10,-10,-10,-10,-20,
       -10,  0,  0,  0,  0,  0,  0,-10,
       -10,  0,  5, 10, 10,  5,  0,-10,
       -10,  5,  5, 10, 10,  5,  5,-10,
       -10,  0, 10, 10, 10, 10,  0,-10,
       -10, 10, 10, 10, 10, 10, 10,-10,
       -10,  5,  0,  0,  0,  0,  5,-10,
       -20,-10,-10,-10,-10,-10,-10,-20,
    ],
    // Rook (index 3)
    [
         0,  0,  0,  0,  0,  0,  0,  0,
         5, 10, 10, 10, 10, 10, 10,  5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
         0,  0,  0,  5,  5,  0,  0,  0,
    ],
    // Queen (index 4)
    [
       -20,-10,-10, -5, -5,-10,-10,-20,
       -10,  0,  0,  0,  0,  0,  0,-10,
       -10,  0,  5,  5,  5,  5,  0,-10,
        -5,  0,  5,  5,  5,  5,  0, -5,
         0,  0,  5,  5,  5,  5,  0, -5,
       -10,  5,  5,  5,  5,  5,  0,-10,
       -10,  0,  5,  0,  0,  0,  0,-10,
       -20,-10,-10, -5, -5,-10,-10,-20,
    ],
    // King (index 5) — middlegame: prefer castled position, avoid center
    [
       -30,-40,-40,-50,-50,-40,-40,-30,
       -30,-40,-40,-50,-50,-40,-40,-30,
       -30,-40,-40,-50,-50,-40,-40,-30,
       -30,-40,-40,-50,-50,-40,-40,-30,
       -20,-30,-30,-40,-40,-30,-30,-20,
       -10,-20,-20,-20,-20,-20,-20,-10,
        20, 20,  0,  0,  0,  0, 20, 20,
        20, 30, 10,  0,  0, 10, 30, 20,
    ],
];

/// Endgame piece-square tables: [piece_index][square]
#[rustfmt::skip]
static PST_EG: [[i32; 64]; 6] = [
    // Pawn (index 0) — endgame: passed pawns more valuable
    [
         0,  0,  0,  0,  0,  0,  0,  0,
        80, 80, 80, 80, 80, 80, 80, 80,
        50, 50, 50, 50, 50, 50, 50, 50,
        30, 30, 30, 30, 30, 30, 30, 30,
        20, 20, 20, 20, 20, 20, 20, 20,
        10, 10, 10, 10, 10, 10, 10, 10,
         5,  5,  5,  5,  5,  5,  5,  5,
         0,  0,  0,  0,  0,  0,  0,  0,
    ],
    // Knight (index 1)
    [
       -50,-40,-30,-30,-30,-30,-40,-50,
       -40,-20,  0,  0,  0,  0,-20,-40,
       -30,  0, 10, 15, 15, 10,  0,-30,
       -30,  5, 15, 20, 20, 15,  5,-30,
       -30,  0, 15, 20, 20, 15,  0,-30,
       -30,  5, 10, 15, 15, 10,  5,-30,
       -40,-20,  0,  5,  5,  0,-20,-40,
       -50,-40,-30,-30,-30,-30,-40,-50,
    ],
    // Bishop (index 2)
    [
       -20,-10,-10,-10,-10,-10,-10,-20,
       -10,  0,  0,  0,  0,  0,  0,-10,
       -10,  0,  5, 10, 10,  5,  0,-10,
       -10,  5,  5, 10, 10,  5,  5,-10,
       -10,  0, 10, 10, 10, 10,  0,-10,
       -10, 10, 10, 10, 10, 10, 10,-10,
       -10,  5,  0,  0,  0,  0,  5,-10,
       -20,-10,-10,-10,-10,-10,-10,-20,
    ],
    // Rook (index 3)
    [
         0,  0,  0,  0,  0,  0,  0,  0,
         5, 10, 10, 10, 10, 10, 10,  5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
         0,  0,  0,  5,  5,  0,  0,  0,
    ],
    // Queen (index 4)
    [
       -20,-10,-10, -5, -5,-10,-10,-20,
       -10,  0,  0,  0,  0,  0,  0,-10,
       -10,  0,  5,  5,  5,  5,  0,-10,
        -5,  0,  5,  5,  5,  5,  0, -5,
         0,  0,  5,  5,  5,  5,  0, -5,
       -10,  5,  5,  5,  5,  5,  0,-10,
       -10,  0,  5,  0,  0,  0,  0,-10,
       -20,-10,-10, -5, -5,-10,-10,-20,
    ],
    // King (index 5) — endgame: king should be active and centralized
    [
       -50,-40,-30,-20,-20,-30,-40,-50,
       -30,-20,-10,  0,  0,-10,-20,-30,
       -30,-10, 20, 30, 30, 20,-10,-30,
       -30,-10, 30, 40, 40, 30,-10,-30,
       -30,-10, 30, 40, 40, 30,-10,-30,
       -30,-10, 20, 30, 30, 20,-10,-30,
       -30,-30,  0,  0,  0,  0,-30,-30,
       -50,-30,-30,-30,-30,-30,-30,-50,
    ],
];

// ─── Pawn Structure Constants ────────────────────────────────────────────────

const DOUBLED_PAWN_PENALTY: i32 = -10;
const ISOLATED_PAWN_PENALTY: i32 = -20;
const BACKWARD_PAWN_PENALTY: i32 = -8;
const PASSED_PAWN_BONUS: [i32; 8] = [0, 5, 10, 20, 35, 60, 100, 0]; // by rank (0=rank1, 7=rank8)

// ─── King Safety Constants ───────────────────────────────────────────────────

const OPEN_FILE_NEAR_KING_PENALTY: i32 = -25;
const PAWN_SHELTER_BONUS: i32 = 10;

// ─── Mobility Constants ──────────────────────────────────────────────────────

const MOBILITY_WEIGHT: i32 = 4; // centipawns per legal square

// ─── Helper: mirror square for Black ─────────────────────────────────────────

/// Mirrors a square index vertically (flips rank). a1(0) <-> a8(56), etc.
#[inline]
fn mirror_sq(sq: u8) -> u8 {
    sq ^ 56
}

// ─── File/Rank helpers ───────────────────────────────────────────────────────

const FILE_A: u64 = 0x0101_0101_0101_0101;

#[inline]
fn file_mask(file: u8) -> u64 {
    FILE_A << file
}

// ─── Evaluation Functions ────────────────────────────────────────────────────

/// Computes material balance: white material minus black material.
fn material_balance(board: &Board) -> i32 {
    let mut score = 0i32;
    for &piece in &[Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen] {
        let white_count = board.pieces[Color::White.index()][piece.index()].count_ones() as i32;
        let black_count = board.pieces[Color::Black.index()][piece.index()].count_ones() as i32;
        score += (white_count - black_count) * piece_value(piece);
    }
    score
}

/// Computes tapered piece-square table score.
/// `phase` is in [0, 24] where 24 = opening, 0 = endgame.
/// Formula: (mg * phase + eg * (24 - phase)) / 24
fn piece_square_score(board: &Board, phase: i32) -> i32 {
    let mut mg = 0i32;
    let mut eg = 0i32;

    for piece_idx in 0..6 {
        // White pieces
        let mut bb = board.pieces[Color::White.index()][piece_idx];
        while bb != 0 {
            let sq = bb.trailing_zeros() as u8;
            mg += PST_MG[piece_idx][sq as usize];
            eg += PST_EG[piece_idx][sq as usize];
            bb &= bb - 1;
        }

        // Black pieces: mirror the square for table lookup, then negate
        let mut bb = board.pieces[Color::Black.index()][piece_idx];
        while bb != 0 {
            let sq = bb.trailing_zeros() as u8;
            let mirrored = mirror_sq(sq);
            mg -= PST_MG[piece_idx][mirrored as usize];
            eg -= PST_EG[piece_idx][mirrored as usize];
            bb &= bb - 1;
        }
    }

    (mg * phase + eg * (24 - phase)) / 24
}

/// Evaluates king safety: penalizes open files near king, rewards pawn shelter.
fn king_safety(board: &Board) -> i32 {
    let mut score = 0i32;

    for &color in &[Color::White, Color::Black] {
        let sign = if color == Color::White { 1 } else { -1 };
        let king_bb = board.pieces[color.index()][Piece::King.index()];
        if king_bb == 0 {
            continue;
        }
        let king_sq = king_bb.trailing_zeros() as u8;
        let king_file = king_sq % 8;
        let our_pawns = board.pieces[color.index()][Piece::Pawn.index()];

        // Check files around the king (king_file-1, king_file, king_file+1)
        let file_start = if king_file > 0 { king_file - 1 } else { 0 };
        let file_end = if king_file < 7 { king_file + 1 } else { 7 };

        for f in file_start..=file_end {
            let fmask = file_mask(f);
            if our_pawns & fmask == 0 {
                // Open file near king — penalty
                score += sign * OPEN_FILE_NEAR_KING_PENALTY;
            } else {
                // Pawn shelter — bonus
                score += sign * PAWN_SHELTER_BONUS;
            }
        }
    }

    score
}

/// Evaluates pawn structure: doubled, isolated, backward, and passed pawns.
fn pawn_structure(board: &Board) -> i32 {
    let mut score = 0i32;

    for &color in &[Color::White, Color::Black] {
        let sign = if color == Color::White { 1 } else { -1 };
        let our_pawns = board.pieces[color.index()][Piece::Pawn.index()];
        let their_pawns = board.pieces[color.opposite().index()][Piece::Pawn.index()];

        for file in 0..8u8 {
            let fmask = file_mask(file);
            let pawns_on_file = our_pawns & fmask;
            let count = pawns_on_file.count_ones() as i32;

            // Doubled pawns: more than one pawn on the same file
            if count > 1 {
                score += sign * DOUBLED_PAWN_PENALTY * (count - 1);
            }

            // Isolated pawns: no friendly pawns on adjacent files
            if count > 0 {
                let has_neighbor = if file > 0 {
                    our_pawns & file_mask(file - 1) != 0
                } else {
                    false
                } || if file < 7 {
                    our_pawns & file_mask(file + 1) != 0
                } else {
                    false
                };

                if !has_neighbor {
                    score += sign * ISOLATED_PAWN_PENALTY * count;
                }
            }
        }

        // Passed pawns and backward pawns — iterate individual pawns
        let mut bb = our_pawns;
        while bb != 0 {
            let sq = bb.trailing_zeros() as u8;
            let file = sq % 8;
            let rank = sq / 8;

            // Passed pawn: no enemy pawns on same or adjacent files ahead
            let is_passed = {
                let mut block_mask = 0u64;
                let file_start = if file > 0 { file - 1 } else { 0 };
                let file_end = if file < 7 { file + 1 } else { 7 };
                for f in file_start..=file_end {
                    let fmask = file_mask(f);
                    // Mask ranks ahead of this pawn
                    if color == Color::White {
                        // Ranks above: clear ranks 0..=rank
                        for r in (rank + 1)..8 {
                            block_mask |= fmask & (0xFFu64 << (r * 8));
                        }
                    } else {
                        // Ranks below: clear ranks rank..=7
                        for r in 0..rank {
                            block_mask |= fmask & (0xFFu64 << (r * 8));
                        }
                    }
                }
                their_pawns & block_mask == 0
            };

            if is_passed {
                let bonus_rank = if color == Color::White { rank } else { 7 - rank };
                score += sign * PASSED_PAWN_BONUS[bonus_rank as usize];
            }

            // Backward pawn: no friendly pawns on adjacent files at same or behind rank,
            // and the stop square is controlled by an enemy pawn
            let is_backward = {
                let has_support = {
                    let mut support = false;
                    if file > 0 {
                        let adj = file_mask(file - 1);
                        // Check for friendly pawns at same rank or behind
                        let behind_mask = if color == Color::White {
                            // Ranks 0..=rank
                            (1u64 << ((rank + 1) * 8)) - 1
                        } else {
                            // Ranks rank..=7
                            !((1u64 << (rank * 8)) - 1)
                        };
                        if our_pawns & adj & behind_mask != 0 {
                            support = true;
                        }
                    }
                    if !support && file < 7 {
                        let adj = file_mask(file + 1);
                        let behind_mask = if color == Color::White {
                            (1u64 << ((rank + 1) * 8)) - 1
                        } else {
                            !((1u64 << (rank * 8)) - 1)
                        };
                        if our_pawns & adj & behind_mask != 0 {
                            support = true;
                        }
                    }
                    support
                };

                if !has_support {
                    // Check if stop square is controlled by enemy pawn
                    let stop_sq = if color == Color::White {
                        if rank < 7 { Some(sq + 8) } else { None }
                    } else {
                        if rank > 0 { Some(sq - 8) } else { None }
                    };
                    if let Some(stop) = stop_sq {
                        magic::pawn_attacks(stop, color) & their_pawns != 0
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if is_backward {
                score += sign * BACKWARD_PAWN_PENALTY;
            }

            bb &= bb - 1;
        }
    }

    score
}

/// Evaluates piece mobility: counts legal squares per piece (excluding pawns and kings).
fn piece_mobility(board: &Board) -> i32 {
    let mut score = 0i32;
    let occ = board.all_occupancy;

    for &color in &[Color::White, Color::Black] {
        let sign = if color == Color::White { 1 } else { -1 };
        let friendly = board.occupancy[color.index()];

        // Knights
        let mut bb = board.pieces[color.index()][Piece::Knight.index()];
        while bb != 0 {
            let sq = bb.trailing_zeros() as u8;
            let moves = magic::knight_attacks(sq) & !friendly;
            score += sign * MOBILITY_WEIGHT * moves.count_ones() as i32;
            bb &= bb - 1;
        }

        // Bishops
        let mut bb = board.pieces[color.index()][Piece::Bishop.index()];
        while bb != 0 {
            let sq = bb.trailing_zeros() as u8;
            let moves = magic::bishop_attacks(sq, occ) & !friendly;
            score += sign * MOBILITY_WEIGHT * moves.count_ones() as i32;
            bb &= bb - 1;
        }

        // Rooks
        let mut bb = board.pieces[color.index()][Piece::Rook.index()];
        while bb != 0 {
            let sq = bb.trailing_zeros() as u8;
            let moves = magic::rook_attacks(sq, occ) & !friendly;
            score += sign * MOBILITY_WEIGHT * moves.count_ones() as i32;
            bb &= bb - 1;
        }

        // Queens
        let mut bb = board.pieces[color.index()][Piece::Queen.index()];
        while bb != 0 {
            let sq = bb.trailing_zeros() as u8;
            let moves = magic::queen_attacks(sq, occ) & !friendly;
            score += sign * MOBILITY_WEIGHT * moves.count_ones() as i32;
            bb &= bb - 1;
        }
    }

    score
}

/// Returns the mate score at a given ply distance.
/// Shorter mates are preferred (higher score).
pub fn mate_score(ply: u32) -> i32 {
    MATE_SCORE - ply as i32
}

/// Main evaluation function. Returns score from the side-to-move's perspective.
/// Positive = good for side to move.
pub fn evaluate(board: &Board) -> i32 {
    let phase = board.game_phase();

    let mut score = 0i32;
    score += material_balance(board);
    score += piece_square_score(board, phase);
    score += king_safety(board);
    score += pawn_structure(board);
    score += piece_mobility(board);

    // Return from side-to-move perspective
    if board.side_to_move == Color::White {
        score
    } else {
        -score
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::magic::init_magic_tables;

    fn setup() {
        init_magic_tables();
    }

    #[test]
    fn starting_position_evaluates_approximately_zero() {
        setup();
        let board = Board::new();
        let score = evaluate(&board);
        // Symmetric position: score should be close to 0
        // Allow some tolerance for PST asymmetry (there shouldn't be any in starting pos)
        assert!(
            score.abs() <= 10,
            "Starting position should evaluate near 0, got {}",
            score
        );
    }

    #[test]
    fn material_balance_extra_white_queen_is_positive() {
        setup();
        // White has an extra queen: standard position but remove black queen
        let board = Board::from_fen(
            "rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        )
        .unwrap();
        let mat = material_balance(&board);
        assert!(
            mat > 0,
            "Material balance with extra white queen should be positive, got {}",
            mat
        );
        assert_eq!(mat, QUEEN_VALUE, "Extra queen should be worth {} cp", QUEEN_VALUE);
    }

    #[test]
    fn mate_score_at_ply_0() {
        assert_eq!(mate_score(0), MATE_SCORE);
    }

    #[test]
    fn mate_score_at_ply_5() {
        assert_eq!(mate_score(5), MATE_SCORE - 5);
    }

    #[test]
    fn piece_value_returns_correct_values() {
        assert_eq!(piece_value(Piece::Pawn), 100);
        assert_eq!(piece_value(Piece::Knight), 320);
        assert_eq!(piece_value(Piece::Bishop), 330);
        assert_eq!(piece_value(Piece::Rook), 500);
        assert_eq!(piece_value(Piece::Queen), 900);
        assert_eq!(piece_value(Piece::King), 0);
    }

    #[test]
    fn evaluate_returns_from_side_to_move_perspective() {
        setup();
        // White has extra queen — evaluate from White's perspective should be positive
        let board_w = Board::from_fen(
            "rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        )
        .unwrap();
        let score_w = evaluate(&board_w);
        assert!(score_w > 0, "White with extra queen, White to move should be positive, got {}", score_w);

        // Same position but Black to move — should be negative (bad for Black)
        let board_b = Board::from_fen(
            "rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
        )
        .unwrap();
        let score_b = evaluate(&board_b);
        assert!(score_b < 0, "White with extra queen, Black to move should be negative, got {}", score_b);
    }

    #[test]
    fn tapered_eval_at_full_phase_uses_mg() {
        setup();
        let board = Board::new();
        let phase = board.game_phase();
        assert_eq!(phase, 24, "Starting position should have phase 24");
        // At phase 24: score = (mg * 24 + eg * 0) / 24 = mg
        let pst = piece_square_score(&board, 24);
        // In starting position, PST should be symmetric = 0
        assert_eq!(pst, 0, "PST score in starting position should be 0, got {}", pst);
    }

    #[test]
    fn material_balance_starting_position_is_zero() {
        let board = Board::new();
        assert_eq!(material_balance(&board), 0);
    }

    #[test]
    fn king_safety_starting_position() {
        setup();
        let board = Board::new();
        let ks = king_safety(&board);
        // Starting position is symmetric, king safety should be 0
        assert_eq!(ks, 0, "King safety in starting position should be 0, got {}", ks);
    }

    #[test]
    fn pawn_structure_starting_position() {
        setup();
        let board = Board::new();
        let ps = pawn_structure(&board);
        // Starting position: no doubled, isolated, or backward pawns; no passed pawns
        // Should be 0 (symmetric)
        assert_eq!(ps, 0, "Pawn structure in starting position should be 0, got {}", ps);
    }

    #[test]
    fn mobility_starting_position_is_zero() {
        setup();
        let board = Board::new();
        let mob = piece_mobility(&board);
        // Starting position is symmetric, mobility should be 0
        assert_eq!(mob, 0, "Mobility in starting position should be 0, got {}", mob);
    }
}
