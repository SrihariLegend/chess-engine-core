// Chaos Theory personality: rewards complexity, penalizes simplification

use crate::board::Board;
use crate::personality::{GameContext, PersonalityEval};

/// Bonus multiplier per total legal move (both sides).
const COMPLEXITY_FACTOR: i32 = 1;

/// Simplification threshold: penalize when total piece count drops below this.
const SIMPLIFICATION_THRESHOLD: u32 = 10;

/// Flat penalty applied when position is simplified below threshold.
const SIMPLIFICATION_PENALTY: i32 = -30;

pub struct ChaosTheory {
    pub weight: f32,
}

impl ChaosTheory {
    pub fn new() -> Self {
        ChaosTheory { weight: 1.0 }
    }
}

impl PersonalityEval for ChaosTheory {
    fn evaluate(&self, board: &Board, ctx: &GameContext) -> i32 {
        let total_moves = ctx.side_to_move_moves + ctx.opponent_moves;
        let mut score = (total_moves as i32) * COMPLEXITY_FACTOR;

        // Penalize simplified positions
        let total_pieces = board.all_occupancy.count_ones();
        if total_pieces < SIMPLIFICATION_THRESHOLD {
            score += SIMPLIFICATION_PENALTY;
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
        "Chaos Theory"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Board, GamePhase};

    fn make_ctx(our_moves: u32, their_moves: u32) -> GameContext {
        GameContext {
            move_number: 1,
            phase: GamePhase::Opening,
            eval_history: [0; 8],
            eval_history_len: 0,
            side_to_move_moves: our_moves,
            opponent_moves: their_moves,
        }
    }

    #[test]
    fn bonus_proportional_to_total_moves() {
        let board = Board::new(); // 32 pieces, above threshold
        let ctx = make_ctx(20, 20);
        let ct = ChaosTheory::new();
        let score = ct.evaluate(&board, &ctx);
        // 40 total moves * 1 = 40
        assert_eq!(score, 40);
    }

    #[test]
    fn more_moves_gives_higher_bonus() {
        let board = Board::new();
        let ct = ChaosTheory::new();
        let score_low = ct.evaluate(&board, &make_ctx(10, 10));
        let score_high = ct.evaluate(&board, &make_ctx(20, 20));
        assert!(score_high > score_low);
    }

    #[test]
    fn simplification_penalty_applied_below_threshold() {
        // Create a board with few pieces (below 10)
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert!(board.all_occupancy.count_ones() < SIMPLIFICATION_THRESHOLD);
        let ctx = make_ctx(5, 5);
        let ct = ChaosTheory::new();
        let score = ct.evaluate(&board, &ctx);
        // 10 * 1 + (-30) = -20
        assert_eq!(score, -20);
    }

    #[test]
    fn no_penalty_above_threshold() {
        let board = Board::new(); // 32 pieces
        let ctx = make_ctx(0, 0);
        let ct = ChaosTheory::new();
        let score = ct.evaluate(&board, &ctx);
        assert_eq!(score, 0); // 0 moves * 2, no penalty
    }

    #[test]
    fn default_weight_is_one() {
        let ct = ChaosTheory::new();
        assert_eq!(ct.weight(), 1.0);
    }

    #[test]
    fn name_is_correct() {
        let ct = ChaosTheory::new();
        assert_eq!(ct.name(), "Chaos Theory");
    }
}
