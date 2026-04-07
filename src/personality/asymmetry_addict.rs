// Asymmetry Addict personality: rewards pawn file asymmetry, penalizes symmetry

use crate::board::{Board, Color, Piece};
use crate::personality::{GameContext, PersonalityEval};

/// Bonus per asymmetric pawn file (we have pawn, opponent doesn't).
const ASYMMETRY_BONUS: i32 = 5;

/// Bonus for bishop pair (relative: + if we have, - if they have).
const BISHOP_PAIR_BONUS: i32 = 15;

/// Bonus for rook vs two minors imbalance.
const IMBALANCE_BONUS: i32 = 15;

/// File masks for pawn file occupancy comparison.
const FILE_MASKS: [u64; 8] = [
    0x0101_0101_0101_0101, // a-file
    0x0202_0202_0202_0202, // b-file
    0x0404_0404_0404_0404, // c-file
    0x0808_0808_0808_0808, // d-file
    0x1010_1010_1010_1010, // e-file
    0x2020_2020_2020_2020, // f-file
    0x4040_4040_4040_4040, // g-file
    0x8080_8080_8080_8080, // h-file
];

pub struct AsymmetryAddict {
    pub weight: f32,
}

impl AsymmetryAddict {
    pub fn new() -> Self {
        AsymmetryAddict { weight: 1.0 }
    }
}

impl PersonalityEval for AsymmetryAddict {
    fn evaluate(&self, board: &Board, _ctx: &GameContext) -> i32 {
        let mut score = 0i32;

        let white_pawns = board.pieces[Color::White.index()][Piece::Pawn.index()];
        let black_pawns = board.pieces[Color::Black.index()][Piece::Pawn.index()];

        // Asymmetry bonus: reward files where we have a pawn and opponent doesn't
        let stm = board.side_to_move;
        let opponent = stm.opposite();
        let stm_pawns = if stm == Color::White { white_pawns } else { black_pawns };
        let opp_pawns = if opponent == Color::White { white_pawns } else { black_pawns };

        let mut asymmetry_score = 0i32;
        for &mask in &FILE_MASKS {
            let stm_on_file = (stm_pawns & mask) != 0;
            let opp_on_file = (opp_pawns & mask) != 0;
            if stm_on_file && !opp_on_file {
                asymmetry_score += 1;
            } else if !stm_on_file && opp_on_file {
                asymmetry_score -= 1;
            }
        }
        score += ASYMMETRY_BONUS * asymmetry_score;

        // Material imbalance bonuses (relative to side to move)
        let our_bishops = board.pieces[stm.index()][Piece::Bishop.index()].count_ones();
        let our_knights = board.pieces[stm.index()][Piece::Knight.index()].count_ones();
        let their_bishops = board.pieces[opponent.index()][Piece::Bishop.index()].count_ones();
        let their_knights = board.pieces[opponent.index()][Piece::Knight.index()].count_ones();
        let our_rooks = board.pieces[stm.index()][Piece::Rook.index()].count_ones();
        let their_rooks = board.pieces[opponent.index()][Piece::Rook.index()].count_ones();

        // Bishop pair bonus: relative (we have 2 - they have 2) * BONUS
        let our_bishop_pair = (our_bishops >= 2) as i32;
        let their_bishop_pair = (their_bishops >= 2) as i32;
        score += (our_bishop_pair - their_bishop_pair) * BISHOP_PAIR_BONUS;

        // Rook vs two minors imbalance: we have rook, opponent has 2+ minors, opponent has no rooks
        let our_minors = our_bishops + our_knights;
        let their_minors = their_bishops + their_knights;
        if our_rooks >= 1 && their_minors >= 2 && their_rooks == 0 {
            score += IMBALANCE_BONUS;
        }
        if their_rooks >= 1 && our_minors >= 2 && our_rooks == 0 {
            score += IMBALANCE_BONUS;
        }

        score
    }

    fn weight(&self) -> f32 {
        self.weight
    }

    fn set_weight(&mut self, w: f32) {
        self.weight = w;
    }

    fn name(&self) -> &str {
        "Asymmetry Addict"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Board, GamePhase};

    fn default_ctx() -> GameContext {
        GameContext {
            move_number: 1,
            phase: GamePhase::Opening,
            eval_history: [0; 8],
            eval_history_len: 0,
            side_to_move_moves: 20,
            opponent_moves: 20,
        }
    }

    #[test]
    fn starting_position_has_symmetry_penalty() {
        let board = Board::new();
        let ctx = default_ctx();
        let aa = AsymmetryAddict::new();
        let score = aa.evaluate(&board, &ctx);
        // Starting position: White to move
        // Pawns: 8 files with both sides have pawns = 0 asymmetry
        // Both sides have 2 bishops = bishop pair cancels (0)
        // Score should be 0
        assert_eq!(score, 0);
    }

    #[test]
    fn no_pawns_no_symmetry_penalty() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let ctx = default_ctx();
        let aa = AsymmetryAddict::new();
        let score = aa.evaluate(&board, &ctx);
        // No pawns, no bishops, no rooks → score = 0
        assert_eq!(score, 0);
    }

    #[test]
    fn bishop_pair_bonus_applied() {
        // White has 2 bishops, black has none, white to move
        let board = Board::from_fen("4k3/8/8/8/8/8/8/2B1KB2 w - - 0 1").unwrap();
        let ctx = default_ctx();
        let aa = AsymmetryAddict::new();
        let score = aa.evaluate(&board, &ctx);
        // White has bishop pair (+15), black doesn't (-0) = +15
        assert_eq!(score, 15);
    }

    #[test]
    fn asymmetry_bonus_proportional_to_files() {
        // Pawns on 2 files where only white has pawns (asymmetry)
        let board = Board::from_fen("4k3/8/8/8/8/8/P1P5/4K3 w - - 0 1").unwrap();
        let ctx = default_ctx();
        let aa = AsymmetryAddict::new();
        let score = aa.evaluate(&board, &ctx);
        // White has pawns on a2,b2, black has none → 2 asymmetric files * 5 = 10
        assert_eq!(score, 10);
    }

    #[test]
    fn default_weight_is_one() {
        let aa = AsymmetryAddict::new();
        assert_eq!(aa.weight(), 1.0);
    }

    #[test]
    fn name_is_correct() {
        let aa = AsymmetryAddict::new();
        assert_eq!(aa.name(), "Asymmetry Addict");
    }
}
