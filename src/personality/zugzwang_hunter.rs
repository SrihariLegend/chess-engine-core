// Zugzwang Hunter personality: seeks positions with few opponent moves

use crate::board::{Board, GamePhase};
use crate::personality::{GameContext, PersonalityEval};

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
        // Bonus inversely proportional to opponent's legal move count
        // Clamp denominator to min 1
        let opp_moves = ctx.opponent_moves.max(1) as i32;
        let mut bonus = ZUGZWANG_BASE / opp_moves;

        // Increased weight during Endgame phase
        if ctx.phase == GamePhase::Endgame {
            bonus *= ENDGAME_MULTIPLIER;
        }

        bonus
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
        // 40/2 = 20 vs 40/20 = 2
        assert_eq!(score_few, 20);
        assert_eq!(score_many, 2);
        assert!(score_few > score_many);
    }

    #[test]
    fn zero_opponent_moves_clamped_to_one() {
        let board = Board::new();
        let zh = ZugzwangHunter::new();
        let score = zh.evaluate(&board, &make_ctx(0, GamePhase::Opening));
        // 40 / max(0, 1) = 40
        assert_eq!(score, 40);
    }

    #[test]
    fn endgame_multiplier_applied() {
        let board = Board::new();
        let zh = ZugzwangHunter::new();
        let score_opening = zh.evaluate(&board, &make_ctx(10, GamePhase::Opening));
        let score_endgame = zh.evaluate(&board, &make_ctx(10, GamePhase::Endgame));
        // Opening: 40/10 = 4, Endgame: 40/10 * 3 = 12
        assert_eq!(score_opening, 4);
        assert_eq!(score_endgame, 12);
        assert_eq!(score_endgame, score_opening * ENDGAME_MULTIPLIER);
    }

    #[test]
    fn non_endgame_phases_no_multiplier() {
        let board = Board::new();
        let zh = ZugzwangHunter::new();
        let phases = [GamePhase::Opening, GamePhase::EarlyMiddlegame, GamePhase::LateMiddlegame];
        for phase in phases {
            let score = zh.evaluate(&board, &make_ctx(10, phase));
            assert_eq!(score, 4, "Non-endgame phase {:?} should give 4", phase);
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
