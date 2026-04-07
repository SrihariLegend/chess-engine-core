// Momentum Tracker personality: adjusts aggression based on eval trend

use crate::board::Board;
use crate::personality::{GameContext, PersonalityEval};

/// Scaling factor for momentum bonus/penalty.
const MOMENTUM_FACTOR: i32 = 2;

pub struct MomentumTracker {
    pub weight: f32,
}

impl MomentumTracker {
    pub fn new() -> Self {
        MomentumTracker { weight: 1.0 }
    }
}

impl PersonalityEval for MomentumTracker {
    fn evaluate(&self, _board: &Board, ctx: &GameContext) -> i32 {
        // Note: Momentum uses past total evaluations, which include momentum itself.
        // This can amplify trends but is intentional for personality expression.
        ctx.momentum() * MOMENTUM_FACTOR
    }

    fn weight(&self) -> f32 {
        self.weight
    }

    fn set_weight(&mut self, w: f32) {
        self.weight = w;
    }

    fn name(&self) -> &str {
        "Momentum Tracker"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Board, GamePhase};

    fn make_ctx_with_evals(evals: &[i32]) -> GameContext {
        let mut ctx = GameContext {
            move_number: 1,
            phase: GamePhase::Opening,
            eval_history: [0; 8],
            eval_history_len: 0,
            side_to_move_moves: 20,
            opponent_moves: 20,
        };
        for &e in evals {
            ctx.push_eval(e);
        }
        ctx
    }

    #[test]
    fn positive_momentum_gives_positive_score() {
        let board = Board::new();
        let mt = MomentumTracker::new();
        // Increasing evals: 0, 10, 20, 30 → slope = 10
        let ctx = make_ctx_with_evals(&[0, 10, 20, 30]);
        let score = mt.evaluate(&board, &ctx);
        // momentum = 10, score = 10 * 2 = 20
        assert_eq!(score, 20);
    }

    #[test]
    fn negative_momentum_gives_negative_score() {
        let board = Board::new();
        let mt = MomentumTracker::new();
        // Decreasing evals: 30, 20, 10, 0 → slope = -10
        let ctx = make_ctx_with_evals(&[30, 20, 10, 0]);
        let score = mt.evaluate(&board, &ctx);
        // momentum = -10, score = -10 * 2 = -20
        assert_eq!(score, -20);
    }

    #[test]
    fn flat_trend_gives_zero() {
        let board = Board::new();
        let mt = MomentumTracker::new();
        let ctx = make_ctx_with_evals(&[50, 50, 50, 50]);
        let score = mt.evaluate(&board, &ctx);
        assert_eq!(score, 0);
    }

    #[test]
    fn no_history_gives_zero() {
        let board = Board::new();
        let mt = MomentumTracker::new();
        let ctx = make_ctx_with_evals(&[]);
        let score = mt.evaluate(&board, &ctx);
        assert_eq!(score, 0);
    }

    #[test]
    fn single_eval_gives_zero() {
        let board = Board::new();
        let mt = MomentumTracker::new();
        let ctx = make_ctx_with_evals(&[100]);
        let score = mt.evaluate(&board, &ctx);
        assert_eq!(score, 0);
    }

    #[test]
    fn default_weight_is_one() {
        let mt = MomentumTracker::new();
        assert_eq!(mt.weight(), 1.0);
    }

    #[test]
    fn name_is_correct() {
        let mt = MomentumTracker::new();
        assert_eq!(mt.name(), "Momentum Tracker");
    }
}
