// Entropy Maximizer personality: rewards move count asymmetry

use crate::board::Board;
use crate::personality::{squash_to_cp, GameContext, PersonalityEval};

/// Bonus per move advantage over opponent.
const ENTROPY_FACTOR: i32 = 3;

pub struct EntropyMaximizer {
    pub weight: f32,
}

impl EntropyMaximizer {
    pub fn new() -> Self {
        EntropyMaximizer { weight: 1.0 }
    }
}

impl PersonalityEval for EntropyMaximizer {
    fn evaluate(&self, _board: &Board, ctx: &GameContext) -> i32 {
        let diff = ctx.side_to_move_moves as i32 - ctx.opponent_moves as i32;
        squash_to_cp(diff as f32 * ENTROPY_FACTOR as f32, 90.0)
    }

    fn weight(&self) -> f32 {
        self.weight
    }

    fn set_weight(&mut self, w: f32) {
        self.weight = w;
    }

    fn name(&self) -> &str {
        "Entropy Maximizer"
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
    fn positive_when_we_have_more_moves() {
        let board = Board::new();
        let em = EntropyMaximizer::new();
        let score = em.evaluate(&board, &make_ctx(30, 20));
        assert!(score > 0, "More moves should give positive score, got {}", score);
        assert!(score <= 100, "Score should be in [-100, 100], got {}", score);
    }

    #[test]
    fn negative_when_opponent_has_more_moves() {
        let board = Board::new();
        let em = EntropyMaximizer::new();
        let score = em.evaluate(&board, &make_ctx(10, 25));
        assert!(score < 0, "Fewer moves should give negative score, got {}", score);
        assert!(score >= -100, "Score should be in [-100, 100], got {}", score);
    }

    #[test]
    fn zero_when_equal_moves() {
        let board = Board::new();
        let em = EntropyMaximizer::new();
        let score = em.evaluate(&board, &make_ctx(20, 20));
        assert_eq!(score, 0);
    }

    #[test]
    fn proportional_to_difference() {
        let board = Board::new();
        let em = EntropyMaximizer::new();
        let score1 = em.evaluate(&board, &make_ctx(25, 20));
        let score2 = em.evaluate(&board, &make_ctx(30, 20));
        assert!(score2 > score1,
            "Larger advantage (30-20={}) should score higher than (25-20={})", score2, score1);
    }

    #[test]
    fn default_weight_is_one() {
        let em = EntropyMaximizer::new();
        assert_eq!(em.weight(), 1.0);
    }

    #[test]
    fn name_is_correct() {
        let em = EntropyMaximizer::new();
        assert_eq!(em.name(), "Entropy Maximizer");
    }
}
