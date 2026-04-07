pub mod chaos_theory;
pub mod romantic;
pub mod entropy_maximizer;
pub mod asymmetry_addict;
pub mod momentum_tracker;
pub mod zugzwang_hunter;

// Re-exports for convenience
pub use chaos_theory::ChaosTheory;
pub use romantic::Romantic;
pub use entropy_maximizer::EntropyMaximizer;
pub use asymmetry_addict::AsymmetryAddict;
pub use momentum_tracker::MomentumTracker;
pub use zugzwang_hunter::ZugzwangHunter;

use crate::board::{Board, GamePhase, Piece};
use crate::eval::piece_value;

// ─── Constants ───────────────────────────────────────────────────────────────

/// Number of personality modules.
pub const NUM_PERSONALITIES: usize = 6;

// Personality indices: 0=Chaos, 1=Romantic, 2=Entropy, 3=Asymmetry, 4=Momentum, 5=Zugzwang
const CHAOS: usize = 0;
const ENTROPY: usize = 2;
const MOMENTUM: usize = 4;
const ZUGZWANG: usize = 5;

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

// ─── GameArc ─────────────────────────────────────────────────────────────────

/// Game arc weight table: maps GamePhase to per-personality weight multipliers.
/// Personality indices: 0=Chaos, 1=Romantic, 2=Entropy, 3=Asymmetry, 4=Momentum, 5=Zugzwang
pub struct GameArc {
    /// weights[phase_index][personality_index]
    pub weights: [[f32; NUM_PERSONALITIES]; 4],
}

impl GameArc {
    /// Returns the default weight profiles per phase.
    pub fn default_arc() -> Self {
        GameArc {
            weights: [
                // Opening:       Chaos  Romantic Entropy Asymmetry Momentum Zugzwang
                [0.5, 1.2, 0.5, 0.8, 0.3, 0.1],
                // EarlyMiddlegame:
                [1.2, 0.8, 1.2, 0.8, 0.5, 0.3],
                // LateMiddlegame:
                [0.8, 0.5, 0.8, 0.5, 1.2, 0.5],
                // Endgame:
                [0.3, 0.3, 0.5, 0.3, 1.0, 1.5],
            ],
        }
    }

    /// Get the phase weight for a given phase and personality index.
    pub fn get_weight(&self, phase: GamePhase, personality_idx: usize) -> f32 {
        let phase_idx = match phase {
            GamePhase::Opening => 0,
            GamePhase::EarlyMiddlegame => 1,
            GamePhase::LateMiddlegame => 2,
            GamePhase::Endgame => 3,
        };
        self.weights[phase_idx][personality_idx]
    }

    /// Set the phase weight for a given phase and personality index.
    pub fn set_weight(&mut self, phase: GamePhase, personality_idx: usize, w: f32) {
        let phase_idx = match phase {
            GamePhase::Opening => 0,
            GamePhase::EarlyMiddlegame => 1,
            GamePhase::LateMiddlegame => 2,
            GamePhase::Endgame => 3,
        };
        self.weights[phase_idx][personality_idx] = w;
    }
}

// ─── Personality Summation ───────────────────────────────────────────────────

/// Compute the total personality contribution to evaluation.
/// Formula: sum(weight_i * phase_weight_i * personality_i.evaluate(board, ctx))
pub fn personality_score(
    board: &Board,
    ctx: &GameContext,
    personalities: &[Box<dyn PersonalityEval>],
    arc: &GameArc,
) -> i32 {
    let mut total = 0.0f32;
    for (i, p) in personalities.iter().enumerate() {
        let eval = p.evaluate(board, ctx) as f32;
        let weight = p.weight();
        let phase_weight = arc.get_weight(ctx.phase, i);
        total += weight * phase_weight * eval;
    }
    total as i32
}

// ─── Dynamic Weight Update ───────────────────────────────────────────────────

/// Compute net material for the side to move in centipawns.
fn net_material(board: &Board) -> f32 {
    let us = board.side_to_move.index();
    let them = board.side_to_move.opposite().index();
    let mut score = 0i32;
    for &piece in &[Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen] {
        let ours = board.pieces[us][piece.index()].count_ones() as i32;
        let theirs = board.pieces[them][piece.index()].count_ones() as i32;
        score += (ours - theirs) * piece_value(piece);
    }
    score as f32 / 100.0
}

/// State-only dynamic weight adjustment. Reads board material, move counts,
/// and piece density to adjust personality weights each move. No circular
/// dependency — never uses the engine's own evaluation.
pub fn update_dynamic_weights(
    board: &Board,
    ctx: &GameContext,
    arc: &GameArc,
    personalities: &mut [Box<dyn PersonalityEval>],
) {
    let net = net_material(board);
    let move_adv = (ctx.side_to_move_moves as f32 - ctx.opponent_moves as f32) / 20.0;
    let total_pieces = board.all_occupancy.count_ones() as f32;
    let endgame_factor = ((16.0 - total_pieces) / 16.0).clamp(0.0, 1.0);

    let mut deltas = [0.0f32; NUM_PERSONALITIES];

    // 1. Material advantage
    if net > 0.0 {
        let capped = net.clamp(0.0, 2.0);
        deltas[ZUGZWANG] += 0.1 * capped;
        deltas[CHAOS] -= 0.05 * capped;
    } else if net < 0.0 {
        let capped = (-net).clamp(0.0, 2.0);
        deltas[CHAOS] += 0.1 * capped;
        deltas[ZUGZWANG] -= 0.05;
    }

    // 2. Move count disparity
    deltas[ENTROPY] += 0.1 * move_adv;
    deltas[CHAOS] += 0.05 * move_adv;

    // 3. Piece density (endgame factor)
    deltas[ZUGZWANG] += 0.2 * endgame_factor;
    deltas[MOMENTUM] += 0.1 * endgame_factor;

    // Combine: baseline + deltas, clamp, then smooth
    for (i, p) in personalities.iter_mut().enumerate() {
        let baseline = arc.get_weight(ctx.phase, i);
        let target = (baseline + deltas[i]).clamp(0.2, 2.5);
        let smoothed = 0.9 * p.weight() + 0.1 * target;
        p.set_weight(smoothed);
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

    #[test]
    fn game_arc_default_values() {
        let arc = GameArc::default_arc();

        // Opening
        assert_eq!(arc.get_weight(GamePhase::Opening, 0), 0.5);  // Chaos
        assert_eq!(arc.get_weight(GamePhase::Opening, 1), 1.2);  // Romantic
        assert_eq!(arc.get_weight(GamePhase::Opening, 2), 0.5);  // Entropy
        assert_eq!(arc.get_weight(GamePhase::Opening, 3), 0.8);  // Asymmetry
        assert_eq!(arc.get_weight(GamePhase::Opening, 4), 0.3);  // Momentum
        assert_eq!(arc.get_weight(GamePhase::Opening, 5), 0.1);  // Zugzwang

        // Endgame
        assert_eq!(arc.get_weight(GamePhase::Endgame, 0), 0.3);
        assert_eq!(arc.get_weight(GamePhase::Endgame, 1), 0.3);
        assert_eq!(arc.get_weight(GamePhase::Endgame, 2), 0.5);
        assert_eq!(arc.get_weight(GamePhase::Endgame, 3), 0.3);
        assert_eq!(arc.get_weight(GamePhase::Endgame, 4), 1.0);
        assert_eq!(arc.get_weight(GamePhase::Endgame, 5), 1.5);
    }

    #[test]
    fn game_arc_set_weight() {
        let mut arc = GameArc::default_arc();
        arc.set_weight(GamePhase::Opening, 0, 2.0);
        assert_eq!(arc.get_weight(GamePhase::Opening, 0), 2.0);
    }

    // Test personality_score with mock personalities
    struct MockPersonality {
        eval_value: i32,
        w: f32,
        n: &'static str,
    }

    impl PersonalityEval for MockPersonality {
        fn evaluate(&self, _board: &Board, _ctx: &GameContext) -> i32 {
            self.eval_value
        }
        fn weight(&self) -> f32 {
            self.w
        }
        fn set_weight(&mut self, w: f32) {
            self.w = w;
        }
        fn name(&self) -> &str {
            self.n
        }
    }

    #[test]
    fn personality_score_sums_correctly() {
        let board = Board::new();
        let ctx = GameContext::new(); // Opening phase

        let personalities: Vec<Box<dyn PersonalityEval>> = vec![
            Box::new(MockPersonality { eval_value: 100, w: 1.0, n: "chaos" }),
            Box::new(MockPersonality { eval_value: 50, w: 1.0, n: "romantic" }),
            Box::new(MockPersonality { eval_value: 0, w: 1.0, n: "entropy" }),
            Box::new(MockPersonality { eval_value: 0, w: 1.0, n: "asymmetry" }),
            Box::new(MockPersonality { eval_value: 0, w: 1.0, n: "momentum" }),
            Box::new(MockPersonality { eval_value: 0, w: 1.0, n: "zugzwang" }),
        ];

        let arc = GameArc::default_arc();
        let score = personality_score(&board, &ctx, &personalities, &arc);

        // Opening phase weights: [0.5, 1.2, 0.5, 0.8, 0.3, 0.1]
        // Expected: 1.0 * 0.5 * 100 + 1.0 * 1.2 * 50 + 0 + 0 + 0 + 0 = 50 + 60 = 110
        assert_eq!(score, 110);
    }

    #[test]
    fn personality_score_with_zero_weights() {
        let board = Board::new();
        let ctx = GameContext::new();

        let personalities: Vec<Box<dyn PersonalityEval>> = vec![
            Box::new(MockPersonality { eval_value: 100, w: 0.0, n: "chaos" }),
            Box::new(MockPersonality { eval_value: 100, w: 0.0, n: "romantic" }),
            Box::new(MockPersonality { eval_value: 100, w: 0.0, n: "entropy" }),
            Box::new(MockPersonality { eval_value: 100, w: 0.0, n: "asymmetry" }),
            Box::new(MockPersonality { eval_value: 100, w: 0.0, n: "momentum" }),
            Box::new(MockPersonality { eval_value: 100, w: 0.0, n: "zugzwang" }),
        ];

        let arc = GameArc::default_arc();
        let score = personality_score(&board, &ctx, &personalities, &arc);
        assert_eq!(score, 0);
    }
}
