pub mod chaos_theory;
pub mod romantic;
pub mod entropy_maximizer;
pub mod asymmetry_addict;
pub mod momentum_tracker;
pub mod zugzwang_hunter;
pub mod profile;

// Re-exports for convenience
pub use chaos_theory::ChaosTheory;
pub use romantic::Romantic;
pub use entropy_maximizer::EntropyMaximizer;
pub use asymmetry_addict::AsymmetryAddict;
pub use momentum_tracker::MomentumTracker;
pub use zugzwang_hunter::ZugzwangHunter;

use crate::board::{Board, GamePhase};

// ─── Normalization ────────────────────────────────────────────────────────────

/// Squash a raw float to [-1, +1] via tanh and scale to centipawns.
/// `scale` is the raw value at which tanh reaches ~0.76 (i.e., ~76 cp).
/// Values beyond ±5× scale are clamped before tanh.
pub fn squash_to_cp(raw: f32, scale: f32) -> i32 {
    if scale <= 0.0 {
        return 0;
    }
    let clamped = (raw / scale).clamp(-5.0, 5.0);
    (clamped.tanh() * 100.0) as i32
}

// ─── Constants ───────────────────────────────────────────────────────────────

/// Number of personality modules.
pub const NUM_PERSONALITIES: usize = 6;

// Personality indices: 0=Chaos, 1=Romantic, 2=Entropy, 3=Asymmetry, 4=Momentum, 5=Zugzwang
pub(crate) const CHAOS: usize = 0;
pub(crate) const ROMANTIC: usize = 1;
pub(crate) const ENTROPY: usize = 2;
pub(crate) const ASYMMETRY: usize = 3;
pub(crate) const MOMENTUM: usize = 4;
pub(crate) const ZUGZWANG: usize = 5;

// ─── PersonalityEval Trait ───────────────────────────────────────────────────

/// Common trait for personality evaluation modules.
/// Each personality contributes a weighted i32 bonus/penalty to the base evaluation.
///
/// Returns a score from the perspective of the side to move.
/// Positive = good for the player whose turn it is.
pub trait PersonalityEval: Send + Sync {
    /// Evaluate the board from this personality's perspective.
    fn evaluate(&self, board: &Board, ctx: &GameContext) -> i32;
    /// Returns the current weight multiplier for this personality.
    fn weight(&self) -> f32;
    /// Sets the weight multiplier.
    fn set_weight(&mut self, w: f32);
    /// Returns the name of this personality.
    fn name(&self) -> &str;
}

// ─── GameContext ──────────────────────────────────────────────────────────────

/// Game state metadata passed to personality evaluators.
pub struct GameContext {
    pub move_number: u16,
    pub phase: GamePhase,
    /// Circular buffer of the last 8 evaluations.
    pub eval_history: [i32; 8],
    /// Number of evaluations pushed so far (capped display at 8, but tracks total).
    pub eval_history_len: u8,
    /// Number of legal moves available to the side to move.
    pub side_to_move_moves: u32,
    /// Number of legal moves available to the opponent.
    pub opponent_moves: u32,
}

impl GameContext {
    /// Creates a new GameContext at the start of a game.
    pub fn new() -> Self {
        GameContext {
            move_number: 1,
            phase: GamePhase::Opening,
            eval_history: [0; 8],
            eval_history_len: 0,
            side_to_move_moves: 0,
            opponent_moves: 0,
        }
    }

    /// Push an evaluation into the circular buffer.
    pub fn push_eval(&mut self, eval: i32) {
        let idx = (self.eval_history_len % 8) as usize;
        self.eval_history[idx] = eval;
        self.eval_history_len = self.eval_history_len.saturating_add(1);
    }

    /// Compute momentum as the linear regression slope over eval_history.
    /// Returns the slope scaled to integer centipawns.
    /// If fewer than 2 entries, returns 0.
    pub fn momentum(&self) -> i32 {
        let n = (self.eval_history_len.min(8)) as usize;
        if n < 2 {
            return 0;
        }

        // Linear regression: slope = (n * sum(x*y) - sum(x) * sum(y)) / (n * sum(x^2) - sum(x)^2)
        // where x = 0, 1, ..., n-1 and y = eval values in chronological order.
        //
        // We need to read the circular buffer in chronological order.
        let start = if self.eval_history_len <= 8 {
            0
        } else {
            (self.eval_history_len % 8) as usize
        };

        let mut sum_x: i64 = 0;
        let mut sum_y: i64 = 0;
        let mut sum_xy: i64 = 0;
        let mut sum_x2: i64 = 0;

        for i in 0..n {
            let buf_idx = (start + i) % 8;
            let x = i as i64;
            let y = self.eval_history[buf_idx] as i64;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
        }

        let n_i64 = n as i64;
        let denom = n_i64 * sum_x2 - sum_x * sum_x;
        if denom == 0 {
            return 0;
        }

        let numer = n_i64 * sum_xy - sum_x * sum_y;
        // Return slope as integer (truncated)
        (numer / denom) as i32
    }

    /// Update the game phase based on move_number.
    pub fn update_phase(&mut self) {
        self.phase = if self.move_number <= 10 {
            GamePhase::Opening
        } else if self.move_number <= 20 {
            GamePhase::EarlyMiddlegame
        } else if self.move_number <= 30 {
            GamePhase::LateMiddlegame
        } else {
            GamePhase::Endgame
        };
    }
}

impl Default for GameContext {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_context_new_defaults() {
        let ctx = GameContext::new();
        assert_eq!(ctx.move_number, 1);
        assert_eq!(ctx.phase, GamePhase::Opening);
        assert_eq!(ctx.eval_history_len, 0);
        assert_eq!(ctx.side_to_move_moves, 0);
        assert_eq!(ctx.opponent_moves, 0);
    }

    #[test]
    fn push_eval_fills_buffer() {
        let mut ctx = GameContext::new();
        for i in 0..8 {
            ctx.push_eval(i * 10);
        }
        assert_eq!(ctx.eval_history_len, 8);
        assert_eq!(ctx.eval_history, [0, 10, 20, 30, 40, 50, 60, 70]);
    }

    #[test]
    fn push_eval_wraps_around() {
        let mut ctx = GameContext::new();
        for i in 0..10 {
            ctx.push_eval(i * 10);
        }
        assert_eq!(ctx.eval_history_len, 10);
        // Buffer should contain: [80, 90, 20, 30, 40, 50, 60, 70]
        // Index 0 was overwritten by eval #8 (80), index 1 by eval #9 (90)
        assert_eq!(ctx.eval_history[0], 80);
        assert_eq!(ctx.eval_history[1], 90);
        assert_eq!(ctx.eval_history[2], 20);
    }

    #[test]
    fn momentum_with_no_history_is_zero() {
        let ctx = GameContext::new();
        assert_eq!(ctx.momentum(), 0);
    }

    #[test]
    fn momentum_with_one_entry_is_zero() {
        let mut ctx = GameContext::new();
        ctx.push_eval(100);
        assert_eq!(ctx.momentum(), 0);
    }

    #[test]
    fn momentum_increasing_trend() {
        let mut ctx = GameContext::new();
        // Push linearly increasing: 0, 10, 20, 30
        for i in 0..4 {
            ctx.push_eval(i * 10);
        }
        let m = ctx.momentum();
        assert_eq!(m, 10, "Linear increase of 10 per step should give slope 10, got {}", m);
    }

    #[test]
    fn momentum_decreasing_trend() {
        let mut ctx = GameContext::new();
        // Push linearly decreasing: 30, 20, 10, 0
        for i in (0..4).rev() {
            ctx.push_eval(i * 10);
        }
        let m = ctx.momentum();
        assert_eq!(m, -10, "Linear decrease of 10 per step should give slope -10, got {}", m);
    }

    #[test]
    fn momentum_flat_trend() {
        let mut ctx = GameContext::new();
        for _ in 0..4 {
            ctx.push_eval(50);
        }
        assert_eq!(ctx.momentum(), 0);
    }

    #[test]
    fn update_phase_opening() {
        let mut ctx = GameContext::new();
        ctx.move_number = 5;
        ctx.update_phase();
        assert_eq!(ctx.phase, GamePhase::Opening);
    }

    #[test]
    fn update_phase_early_middlegame() {
        let mut ctx = GameContext::new();
        ctx.move_number = 15;
        ctx.update_phase();
        assert_eq!(ctx.phase, GamePhase::EarlyMiddlegame);
    }

    #[test]
    fn update_phase_late_middlegame() {
        let mut ctx = GameContext::new();
        ctx.move_number = 25;
        ctx.update_phase();
        assert_eq!(ctx.phase, GamePhase::LateMiddlegame);
    }

    #[test]
    fn update_phase_endgame() {
        let mut ctx = GameContext::new();
        ctx.move_number = 35;
        ctx.update_phase();
        assert_eq!(ctx.phase, GamePhase::Endgame);
    }

    #[test]
    fn update_phase_boundaries() {
        let mut ctx = GameContext::new();

        ctx.move_number = 10;
        ctx.update_phase();
        assert_eq!(ctx.phase, GamePhase::Opening);

        ctx.move_number = 11;
        ctx.update_phase();
        assert_eq!(ctx.phase, GamePhase::EarlyMiddlegame);

        ctx.move_number = 20;
        ctx.update_phase();
        assert_eq!(ctx.phase, GamePhase::EarlyMiddlegame);

        ctx.move_number = 21;
        ctx.update_phase();
        assert_eq!(ctx.phase, GamePhase::LateMiddlegame);

        ctx.move_number = 30;
        ctx.update_phase();
        assert_eq!(ctx.phase, GamePhase::LateMiddlegame);

        ctx.move_number = 31;
        ctx.update_phase();
        assert_eq!(ctx.phase, GamePhase::Endgame);
    }

}
