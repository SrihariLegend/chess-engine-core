// Entropy Maximizer personality: rewards move count asymmetry

use crate::board::Board;
use crate::personality::{GameContext, PersonalityEval};

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
        (ctx.side_to_move_moves as i32 - ctx.opponent_moves as i32) * ENTROPY_FACTOR
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
        // (30 - 20) * 3 = 30
        assert_eq!(score, 30);
    }

    #[test]
    fn negative_when_opponent_has_more_moves() {
        let board = Board::new();
        let em = EntropyMaximizer::new();
        let score = em.evaluate(&board, &make_ctx(10, 25));
        // (10 - 25) * 3 = -45
        assert_eq!(score, -45);
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
        // score2 should be double score1: 15 vs 30 (factor = 3)
        assert_eq!(score1, 15);
        assert_eq!(score2, 30);
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
