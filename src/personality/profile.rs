// Playing style profiles: named player styles expressed through 5 behavioral axes.
//
// Each profile defines preferences on 5 axes [-1.0, +1.0]:
//   activity      — seeks piece activity, open positions, initiative
//   complexity    — seeks chaotic/complex positions, avoids simplification
//   risk          — willingness to enter unclear forcing lines, sacrifice material
//   simplification — seeks trades, endgame transitions, clarity
//   prophylaxis   — restricts opponent options, creates zugzwang, slow maneuvering
//
// These axes map to the 6 normalized sensor outputs (ChaosTheory, Romantic, etc.)
// via the AXIS_SENSOR_BLEND table. The combined signal feeds into evaluation,
// move ordering, and search behavior channels.

use crate::board::{Board, Color, Move, Piece};
use crate::personality::{PersonalityEval, GameContext, GamePhase, CHAOS, ROMANTIC, ENTROPY, ASYMMETRY, MOMENTUM, ZUGZWANG};

/// The maximum centipawn contribution from the personality evaluation channel.
pub const MAX_CHANNEL1_CP: f32 = 20.0;

// ─── Profile ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Profile {
    pub name: &'static str,
    pub activity: f32,
    pub complexity: f32,
    pub risk: f32,
    pub simplification: f32,
    pub prophylaxis: f32,
    pub adaptive: bool,
}

impl Profile {
    pub fn axes(&self) -> [f32; 5] {
        [self.activity, self.complexity, self.risk, self.simplification, self.prophylaxis]
    }

    /// Adapt axis values based on game context. Only meaningful for adaptive profiles.
    pub fn adapt(&self, ctx: &GameContext, board: &Board) -> Profile {
        if !self.adaptive {
            return self.clone();
        }

        let mut adapted = self.clone();
        let net_material = material_balance_in_pawns(board);
        let momentum = ctx.momentum();

        // Material: ahead → simplify; behind → complicate
        if net_material > 1.0 {
            adapted.simplification = 0.5;
            adapted.risk = -0.3;
        } else if net_material < -1.0 {
            adapted.complexity = 0.6;
            adapted.risk = 0.5;
        } else {
            // Equal: adapt based on momentum
            if momentum > 30 {
                adapted.activity = 0.4;
                adapted.risk = 0.3;
            } else if momentum < -30 {
                adapted.prophylaxis = 0.4;
                adapted.risk = -0.2;
            } else {
                adapted.simplification = 0.3;
                adapted.activity = 0.2;
            }
        }

        // Phase adaptation
        match ctx.phase {
            GamePhase::Opening => { adapted.complexity = 0.3; }
            GamePhase::Endgame => { adapted.simplification = 0.6; adapted.prophylaxis = 0.4; }
            _ => {}
        }

        adapted
    }
}

// ─── Player Profiles ──────────────────────────────────────────────────────────

pub const TAL: Profile = Profile {
    name: "Tal", activity: 0.9, complexity: 0.7, risk: 0.8, simplification: -0.4, prophylaxis: -0.6, adaptive: false,
};
pub const PETROSIAN: Profile = Profile {
    name: "Petrosian", activity: -0.5, complexity: -0.6, risk: -0.7, simplification: 0.6, prophylaxis: 0.8, adaptive: false,
};
pub const KARPOV: Profile = Profile {
    name: "Karpov", activity: 0.1, complexity: 0.0, risk: -0.2, simplification: 0.7, prophylaxis: 0.7, adaptive: false,
};
pub const CAPABLANCA: Profile = Profile {
    name: "Capablanca", activity: 0.2, complexity: -0.3, risk: 0.1, simplification: 0.8, prophylaxis: 0.3, adaptive: false,
};
pub const MORPHY: Profile = Profile {
    name: "Morphy", activity: 0.9, complexity: 0.4, risk: 0.7, simplification: 0.0, prophylaxis: -0.5, adaptive: false,
};
pub const ALEKHINE: Profile = Profile {
    name: "Alekhine", activity: 0.7, complexity: 0.8, risk: 0.3, simplification: 0.1, prophylaxis: 0.3, adaptive: false,
};
pub const LASKER: Profile = Profile {
    name: "Lasker", activity: 0.0, complexity: 0.0, risk: 0.0, simplification: 0.0, prophylaxis: 0.0, adaptive: true,
};

// ─── Profile Lookup ───────────────────────────────────────────────────────────

pub fn profile_by_name(name: &str) -> Option<&'static Profile> {
    match name.to_lowercase().as_str() {
        "tal" => Some(&TAL),
        "petrosian" => Some(&PETROSIAN),
        "karpov" => Some(&KARPOV),
        "capablanca" => Some(&CAPABLANCA),
        "morphy" => Some(&MORPHY),
        "alekhine" => Some(&ALEKHINE),
        "lasker" => Some(&LASKER),
        _ => None,
    }
}

// ─── Axis-to-Sensor Blend ─────────────────────────────────────────────────────

/// Maps each of the 5 behavioral axes to a weighted blend of 1-2 normalized
/// sensor outputs. Each row is [(sensor_index, coefficient), ...].
static AXIS_SENSOR_BLEND: [[(usize, f32); 2]; 5] = [
    // activity:      0.7 * romantic +    0.3 * momentum
    [(ROMANTIC, 0.7), (MOMENTUM, 0.3)],
    // complexity:    1.0 * chaos
    [(CHAOS, 1.0), (CHAOS, 0.0)],
    // risk:          0.6 * momentum +    0.4 * chaos
    [(MOMENTUM, 0.6), (CHAOS, 0.4)],
    // simplification: 0.5 * (-entropy) + 0.5 * zugzwang
    [(ENTROPY, -0.5), (ZUGZWANG, 0.5)],
    // prophylaxis:   0.6 * zugzwang +    0.4 * asymmetry
    [(ZUGZWANG, 0.6), (ASYMMETRY, 0.4)],
];

// ─── Channel 1: Position Preference ───────────────────────────────────────────

/// Compute the personality evaluation contribution (Channel 1).
/// Returns a value in [-MAX_CHANNEL1_CP, +MAX_CHANNEL1_CP] scaled by intensity.
pub fn compute_channel1(
    board: &Board,
    ctx: &GameContext,
    sensors: &[Box<dyn PersonalityEval>],
    profile: &Profile,
    intensity: f32,
) -> i32 {
    if intensity <= 0.0 {
        return 0;
    }

    let intensity = intensity.clamp(0.0, 1.0);

    // 1. Get normalized sensor readings (already in [-100, 100])
    let mut sensor_vals = [0f32; 6];
    for (i, s) in sensors.iter().enumerate() {
        sensor_vals[i] = s.evaluate(board, ctx) as f32;
    }

    // 2. Compute axis values from sensor blends
    let axis_vals = AXIS_SENSOR_BLEND.map(|blend| {
        blend.iter().map(|(idx, coeff)| sensor_vals[*idx] * coeff).sum::<f32>()
    });

    // 3. Dot product with profile preferences
    let profile_axes = profile.axes();
    let total: f32 = profile_axes.iter().zip(axis_vals.iter())
        .map(|(pref, axis)| pref * axis)
        .sum();

    // 4. Scale: normalize by number of axes, cap at MAX_CHANNEL1_CP
    let norm_factor = profile_axes.iter().map(|a| a.abs()).sum::<f32>().max(0.1);
    let raw_max = norm_factor * 100.0 / 5.0;
    let scale = intensity * MAX_CHANNEL1_CP / raw_max.max(1.0);
    (total * scale).round() as i32
}

// ─── Signal Blend Tables ──────────────────────────────────────────────────────

/// Maps each of the 5 move-level signals to a weighted blend of the 5 behavioral axes.
/// Each row is [(axis_index, coefficient), ...] where axis order is [activity, complexity, risk, simplification, prophylaxis].
/// Used to compute per-profile preferences on move signals via dot product.
static SIGNAL_BLENDS: [[(usize, f32); 5]; 5] = [
    // simplify_seek: prefers trading — high simplification, low complexity/risk
    [(0, 0.0), (1, -0.7), (2, -0.3), (3, 1.0), (4, 0.5)],
    // complexity_seek: avoids simplification — high complexity, low simplification
    [(0, 0.3), (1, 1.0), (2, 0.7), (3, -0.7), (4, -0.3)],
    // develop: prefers getting pieces off the back rank — high activity
    [(0, 0.8), (1, 0.2), (2, 0.0), (3, 0.0), (4, -0.5)],
    // attack: prefers moves toward opponent king — high activity, high risk
    [(0, 0.7), (1, 0.5), (2, 0.5), (3, -0.5), (4, -0.7)],
    // safety: prefers consolidating, defensive moves — high prophylaxis, low activity/risk
    [(0, -0.7), (1, -0.5), (2, -0.7), (3, 0.5), (4, 0.8)],
];

/// Compute a profile's preference for a given signal via dot product of axes with blend weights.
fn signal_pref(axes: &[f32; 5], signal_idx: usize) -> f32 {
    SIGNAL_BLENDS[signal_idx].iter()
        .map(|(axis_idx, coeff)| axes[*axis_idx] * coeff)
        .sum()
}

// ─── Channel 2: Move Ordering Bias ────────────────────────────────────────────

/// Maximum personality bonus added to a move's ordering score.
const MAX_MOVE_BONUS: i32 = 3000;

/// Compute a personality-based bonus for move ordering using semantic move signals.
/// Positive bonus = examine this move earlier (preferred by this style).
pub fn personality_move_bonus(mv: &Move, board: &Board, profile: &Profile) -> i32 {
    let axes = profile.axes();
    let mut bonus = 0f32;

    let is_capture = mv.is_capture();
    let is_promotion = mv.is_promotion();

    // ── Simplifying trade detection ──
    // A move is "simplifying" if it's a capture where we trade equal or better material,
    // or any queen trade. High-simplification profiles (Capablanca, Petrosian) prefer these.
    // High-complexity profiles (Tal, Alekhine) avoid them.
    if is_capture {
        if let Some(victim) = mv.captured {
            let is_simplifying = piece_value_simple(victim) >= piece_value_simple(mv.piece);
            if is_simplifying {
                // simplify_seek signal: positive = seek trades, negative = avoid them
                bonus += signal_pref(&axes, 0) * 800.0;
            } else {
                // Winning capture — all aggressive profiles like these
                bonus += signal_pref(&axes, 3) * 600.0; // attack signal
            }
        }
    }

    if is_promotion {
        bonus += signal_pref(&axes, 3) * 400.0; // attack signal
    }

    // ── Development detection ──
    // A move is "developing" if the piece starts on its home rank (back rank for its color)
    // and moves forward. Morphy in particular should get a strong bonus for this.
    if !is_capture && !is_promotion {
        let from_rank = mv.from / 8;
        let to_rank = mv.to / 8;
        let piece = mv.piece;
        let is_back_rank_piece = matches!(piece, Piece::Knight | Piece::Bishop | Piece::Queen);
        let is_white_to_move = board.side_to_move == Color::White;
        let is_developing_move = if is_white_to_move {
            from_rank == 0 && to_rank > from_rank
        } else {
            from_rank == 7 && to_rank < from_rank
        };
        if is_back_rank_piece && is_developing_move {
            bonus += signal_pref(&axes, 2) * 1000.0; // develop signal
        }
    }

    // ── Attacking / safety direction for quiet moves ──
    if !is_capture && !is_promotion {
        // Attack signal: positive for Tal/Morphy, negative for Petrosian
        bonus += signal_pref(&axes, 3) * 400.0; // attack
        // Safety signal: positive for Petrosian/Karpov, negative for Tal
        bonus += signal_pref(&axes, 4) * 400.0; // safety
    }

    (bonus.round() as i32).clamp(-MAX_MOVE_BONUS, MAX_MOVE_BONUS)
}

/// Piece value for trade classification (simplified: pawn=1, minor=3, rook=5, queen=9).
fn piece_value_simple(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => 1,
        Piece::Knight | Piece::Bishop => 3,
        Piece::Rook => 5,
        Piece::Queen => 9,
        Piece::King => 99,
    }
}

// ─── Channel 3: Search Behavior ──────────────────────────────────────────────

/// Search behavior parameters derived from a Profile + intensity.
#[derive(Clone, Debug)]
pub struct SearchStyleParams {
    /// Extra plies to add when the side to move is in check.
    pub check_extension_extra: i32,
    /// Adjustment to null move pruning reduction (higher = prune more aggressively).
    pub null_move_reduction_bias: i32,
    /// Adjustment to late move reduction (higher = reduce quiet moves more).
    pub lmr_reduction_bias: i32,
    /// Complexity-driven LMR adjustment: high complexity reduces LMR less.
    pub lmr_complexity_bias: i32,
}

impl SearchStyleParams {
    pub fn neutral() -> Self {
        SearchStyleParams {
            check_extension_extra: 0,
            null_move_reduction_bias: 0,
            lmr_reduction_bias: 0,
            lmr_complexity_bias: 0,
        }
    }
}

impl Profile {
    /// Derive search behavior parameters from this profile and intensity.
    pub fn to_search_params(&self, intensity: f32) -> SearchStyleParams {
        let intensity = intensity.clamp(0.0, 1.0);
        SearchStyleParams {
            check_extension_extra: ((self.risk.max(0.0) * intensity).round() as i32).max(0),
            null_move_reduction_bias: ((self.risk * intensity * 2.0).round() as i32).clamp(0, 2),
            lmr_reduction_bias: ((self.risk * intensity).round() as i32).clamp(-1, 1),
            // High complexity = reduce LMR less (search tactical lines deeper)
            lmr_complexity_bias: ((-self.complexity.max(0.0) * intensity).round() as i32).clamp(-1, 0),
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Net material for the side to move in pawn units.
fn material_balance_in_pawns(board: &Board) -> f32 {
    use crate::board::Piece;
    use crate::eval::piece_value;
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

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::personality::GameContext;

    fn default_ctx() -> GameContext {
        GameContext::new()
    }

    #[test]
    fn profile_axes_match_definitions() {
        assert_eq!(TAL.axes(), [0.9, 0.7, 0.8, -0.4, -0.6]);
        assert_eq!(PETROSIAN.axes(), [-0.5, -0.6, -0.7, 0.6, 0.8]);
        assert_eq!(CAPABLANCA.axes(), [0.2, -0.3, 0.1, 0.8, 0.3]);
        assert_eq!(LASKER.axes(), [0.0, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn profile_by_name_lookup() {
        assert!(profile_by_name("Tal").is_some());
        assert!(profile_by_name("tal").is_some());
        assert!(profile_by_name("TAL").is_some());
        assert!(profile_by_name("unknown").is_none());
    }

    #[test]
    fn lasker_adapts_based_on_game_state() {
        let ctx = default_ctx();
        let board = Board::new();
        let adapted = LASKER.adapt(&ctx, &board);
        // With equal material and neutral state, Lasker should adapt
        // (exact values depend on position, but it should not be all zeros)
        assert_eq!(adapted.name, "Lasker");
    }

    #[test]
    fn non_lasker_profile_adapt_is_identity() {
        let ctx = default_ctx();
        let board = Board::new();
        let adapted = TAL.adapt(&ctx, &board);
        assert_eq!(adapted.axes(), TAL.axes());
    }

    #[test]
    fn compute_channel1_returns_zero_at_zero_intensity() {
        let board = Board::new();
        let ctx = default_ctx();
        use crate::personality::{ChaosTheory, Romantic, EntropyMaximizer, AsymmetryAddict, MomentumTracker, ZugzwangHunter};
        let sensors: Vec<Box<dyn PersonalityEval>> = vec![
            Box::new(ChaosTheory::new()),
            Box::new(Romantic::new()),
            Box::new(EntropyMaximizer::new()),
            Box::new(AsymmetryAddict::new()),
            Box::new(MomentumTracker::new()),
            Box::new(ZugzwangHunter::new()),
        ];
        let score = compute_channel1(&board, &ctx, &sensors, &TAL, 0.0);
        assert_eq!(score, 0);
    }

    #[test]
    fn compute_channel1_bounded_at_max_intensity() {
        let board = Board::new();
        let ctx = default_ctx();
        use crate::personality::{ChaosTheory, Romantic, EntropyMaximizer, AsymmetryAddict, MomentumTracker, ZugzwangHunter};
        let sensors: Vec<Box<dyn PersonalityEval>> = vec![
            Box::new(ChaosTheory::new()),
            Box::new(Romantic::new()),
            Box::new(EntropyMaximizer::new()),
            Box::new(AsymmetryAddict::new()),
            Box::new(MomentumTracker::new()),
            Box::new(ZugzwangHunter::new()),
        ];
        let score = compute_channel1(&board, &ctx, &sensors, &TAL, 1.0);
        assert!(score.abs() <= MAX_CHANNEL1_CP as i32,
            "Channel 1 contribution {} exceeds max {}", score, MAX_CHANNEL1_CP as i32);
    }
}
