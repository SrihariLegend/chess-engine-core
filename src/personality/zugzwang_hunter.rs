// Zugzwang Hunter personality: seeks positions with few opponent moves

use crate::board::{Board, GamePhase};
use crate::personality::{squash_to_cp, GameContext, PersonalityEval};

/// Base bonus scaling factor.
const ZUGZWANG_BASE: i32 = 40;

/// Endgame weight multiplier.
const ENDGAME_MULTIPLIER: i32 = 3;

pub struct ZugzwangHunter {
    pub weight: f32,
}

impl ZugzwangHunter {
    pub fn new() -> Self {
        ZugzwangHunter { weight: 1.0 }
    }
}

impl PersonalityEval for ZugzwangHunter {
    fn evaluate(&self, _board: &Board, ctx: &GameContext) -> i32 {
        let opp_moves = ctx.opponent_moves.max(1) as f32;
        let mut raw = ZUGZWANG_BASE as f32 / opp_moves;

        if ctx.phase == GamePhase::Endgame {
            raw *= ENDGAME_MULTIPLIER as f32;
        }

        squash_to_cp(raw, 80.0)
    }

    fn weight(&self) -> f32 {
        self.weight
    }

    fn set_weight(&mut self, w: f32) {
        self.weight = w;
    }

    fn name(&self) -> &str {
        "Zugzwang Hunter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Board, GamePhase};

    fn make_ctx(their_moves: u32, phase: GamePhase) -> GameContext {
        GameContext {
            move_number: 1,
            phase,
            eval_history: [0; 8],
            eval_history_len: 0,
            side_to_move_moves: 20,
            opponent_moves: their_moves,
        }
    }

    #[test]
    fn fewer_opponent_moves_gives_higher_bonus() {
        let board = Board::new();
        let zh = ZugzwangHunter::new();
        let score_few = zh.evaluate(&board, &make_ctx(2, GamePhase::Opening));
        let score_many = zh.evaluate(&board, &make_ctx(20, GamePhase::Opening));
        assert!(score_few > score_many, "Fewer opponent moves (2) should score higher than many (20): {} vs {}", score_few, score_many);
        assert!(score_few > 0, "Few moves should give positive bonus, got {}", score_few);
    }

    #[test]
    fn zero_opponent_moves_clamped_to_one() {
        let board = Board::new();
        let zh = ZugzwangHunter::new();
        let score = zh.evaluate(&board, &make_ctx(0, GamePhase::Opening));
        assert!(score > 0, "Zero moves (clamped to 1) should give positive bonus, got {}", score);
        assert!(score <= 100, "Score should be in [-100, 100], got {}", score);
    }

    #[test]
    fn endgame_multiplier_applied() {
        let board = Board::new();
        let zh = ZugzwangHunter::new();
        let score_opening = zh.evaluate(&board, &make_ctx(10, GamePhase::Opening));
        let score_endgame = zh.evaluate(&board, &make_ctx(10, GamePhase::Endgame));
        assert!(score_endgame > score_opening,
            "Endgame score ({}) should be higher than opening ({})", score_endgame, score_opening);
    }

    #[test]
    fn non_endgame_phases_no_multiplier() {
        let board = Board::new();
        let zh = ZugzwangHunter::new();
        let phases = [GamePhase::Opening, GamePhase::EarlyMiddlegame, GamePhase::LateMiddlegame];
        for phase in phases {
            let score = zh.evaluate(&board, &make_ctx(10, phase));
            assert!(score >= 0, "Non-endgame phase {:?} should give non-negative score, got {}", phase, score);
        }
    }

    #[test]
    fn default_weight_is_one() {
        let zh = ZugzwangHunter::new();
        assert_eq!(zh.weight(), 1.0);
    }

    #[test]
    fn name_is_correct() {
        let zh = ZugzwangHunter::new();
        assert_eq!(zh.name(), "Zugzwang Hunter");
    }
}
