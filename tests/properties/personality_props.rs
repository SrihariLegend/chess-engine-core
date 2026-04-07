use proptest::prelude::*;
use chess_engine_core::board::{Board, GamePhase};
use chess_engine_core::personality::{
    GameArc, GameContext, PersonalityEval, NUM_PERSONALITIES, personality_score,
};
use chess_engine_core::personality::chaos_theory::ChaosTheory;
use chess_engine_core::personality::romantic::Romantic;
use chess_engine_core::personality::entropy_maximizer::EntropyMaximizer;
use chess_engine_core::personality::asymmetry_addict::AsymmetryAddict;
use chess_engine_core::personality::momentum_tracker::MomentumTracker;
use chess_engine_core::personality::zugzwang_hunter::ZugzwangHunter;

struct MockPersonality { eval_value: i32, w: f32, n: String }
impl PersonalityEval for MockPersonality {
    fn evaluate(&self, _board: &Board, _ctx: &GameContext) -> i32 { self.eval_value }
    fn weight(&self) -> f32 { self.w }
    fn set_weight(&mut self, w: f32) { self.w = w; }
    fn name(&self) -> &str { &self.n }
}

fn game_phase_strategy() -> impl Strategy<Value = GamePhase> {
    prop_oneof![
        Just(GamePhase::Opening),
        Just(GamePhase::EarlyMiddlegame),
        Just(GamePhase::LateMiddlegame),
        Just(GamePhase::Endgame),
    ]
}

fn move_number_for_phase(phase: GamePhase) -> u16 {
    match phase {
        GamePhase::Opening => 5,
        GamePhase::EarlyMiddlegame => 15,
        GamePhase::LateMiddlegame => 25,
        GamePhase::Endgame => 35,
    }
}

// Feature: chess-engine-core, Property 23: Weighted Personality Summation
// **Validates: Requirements 12.2**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    fn property_23_weighted_personality_summation(
        evals in prop::array::uniform6(-500i32..500i32),
        weights in prop::array::uniform6(0.0f32..2.0f32),
        phase in game_phase_strategy(),
    ) {
        let board = Board::new();
        let mut ctx = GameContext::new();
        ctx.phase = phase;
        ctx.move_number = move_number_for_phase(phase);
        let names = ["chaos", "romantic", "entropy", "asymmetry", "momentum", "zugzwang"];
        let personalities: Vec<Box<dyn PersonalityEval>> = (0..NUM_PERSONALITIES)
            .map(|i| Box::new(MockPersonality {
                eval_value: evals[i], w: weights[i], n: names[i].to_string(),
            }) as Box<dyn PersonalityEval>).collect();
        let arc = GameArc::default_arc();
        let actual = personality_score(&board, &ctx, &personalities, &arc);
        let mut expected_f32 = 0.0f32;
        for i in 0..NUM_PERSONALITIES {
            expected_f32 += weights[i] * arc.get_weight(phase, i) * evals[i] as f32;
        }
        prop_assert_eq!(actual, expected_f32 as i32);
    }
}

// Feature: chess-engine-core, Property 34: Game Arc Phase Weight Profiles
// **Validates: Requirements 22.1, 22.2, 22.3, 22.4**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    fn property_34_game_arc_phase_weight_profiles(
        phase in game_phase_strategy(),
        personality_idx in 0usize..NUM_PERSONALITIES,
    ) {
        let arc = GameArc::default_arc();
        let w = arc.get_weight(phase, personality_idx);
        let tbl: [[f32; 6]; 4] = [
            [0.5, 1.2, 0.5, 0.8, 0.3, 0.1],
            [1.2, 0.8, 1.2, 0.8, 0.5, 0.3],
            [0.8, 0.5, 0.8, 0.5, 1.2, 0.5],
            [0.3, 0.3, 0.5, 0.3, 1.0, 1.5],
        ];
        let pi = match phase {
            GamePhase::Opening => 0, GamePhase::EarlyMiddlegame => 1,
            GamePhase::LateMiddlegame => 2, GamePhase::Endgame => 3,
        };
        prop_assert!((w - tbl[pi][personality_idx]).abs() < f32::EPSILON);
        match phase {
            GamePhase::Opening => { prop_assert!(arc.get_weight(phase, 1) >= 1.0); }
            GamePhase::EarlyMiddlegame => {
                prop_assert!(arc.get_weight(phase, 0) >= 1.0);
                prop_assert!(arc.get_weight(phase, 2) >= 1.0);
            }
            GamePhase::LateMiddlegame => { prop_assert!(arc.get_weight(phase, 4) >= 1.0); }
            GamePhase::Endgame => {
                prop_assert!(arc.get_weight(phase, 5) >= 1.0);
                prop_assert!(arc.get_weight(phase, 4) >= 1.0);
            }
        }
    }
}

// Feature: chess-engine-core, Property 24: Chaos Theory Monotonicity
// **Validates: Requirements 13.2, 13.3**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    fn property_24_chaos_theory_monotonicity(
        our_moves_a in 0u32..60, their_moves_a in 0u32..60, extra_moves in 1u32..30,
    ) {
        let chaos = ChaosTheory::new();
        let board = Board::new();
        let mut ctx_a = GameContext::new();
        ctx_a.side_to_move_moves = our_moves_a;
        ctx_a.opponent_moves = their_moves_a;
        let mut ctx_b = GameContext::new();
        ctx_b.side_to_move_moves = our_moves_a + extra_moves;
        ctx_b.opponent_moves = their_moves_a;
        prop_assert!(chaos.evaluate(&board, &ctx_b) >= chaos.evaluate(&board, &ctx_a));
    }
    #[test]
    fn property_24_chaos_theory_simplification_penalty(
        our_moves in 0u32..40, their_moves in 0u32..40,
    ) {
        let chaos = ChaosTheory::new();
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let mut ctx = GameContext::new();
        ctx.side_to_move_moves = our_moves;
        ctx.opponent_moves = their_moves;
        let score = chaos.evaluate(&board, &ctx);
        
        let expected = (our_moves + their_moves) as i32 * 1 + (-30);
        prop_assert_eq!(score, expected);
    }
}

// Feature: chess-engine-core, Property 25: Romantic Activity Scoring
// **Validates: Requirements 14.2, 14.3**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    fn property_25_romantic_activity_scoring(_dummy in 0u32..100) {
        chess_engine_core::board::magic::init_magic_tables();
        let romantic = Romantic::new();
        let board = Board::new();
        let ctx = GameContext::new();
        let score = romantic.evaluate(&board, &ctx);
        prop_assert!(score.abs() < 10000);
        let open = Board::from_fen("r1bqkb1r/pppppppp/2n2n2/8/4P3/2N2N2/PPPP1PPP/R1BQKB1R w KQkq - 4 3").unwrap();
        prop_assert!(romantic.evaluate(&open, &ctx).abs() < 10000);
    }
    #[test]
    fn property_25_romantic_passive_piece_penalty(_dummy in 0u32..100) {
        chess_engine_core::board::magic::init_magic_tables();
        let romantic = Romantic::new();
        let board = Board::from_fen("4k3/8/8/8/8/1P6/PBP5/4K3 w - - 0 1").unwrap();
        let ctx = GameContext::new();
        prop_assert!(romantic.evaluate(&board, &ctx) < 50);
    }
}

// Feature: chess-engine-core, Property 26: Entropy Maximizer Proportionality
// **Validates: Requirements 15.2**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    fn property_26_entropy_maximizer_proportionality(our_moves in 0u32..60, their_moves in 0u32..60) {
        let e = EntropyMaximizer::new();
        let board = Board::new();
        let mut ctx = GameContext::new();
        ctx.side_to_move_moves = our_moves;
        ctx.opponent_moves = their_moves;
        let score = e.evaluate(&board, &ctx);
        let diff = our_moves as i32 - their_moves as i32;
        prop_assert_eq!(score, 3 * diff);
        if our_moves > their_moves { prop_assert!(score > 0); }
        if their_moves > our_moves { prop_assert!(score < 0); }
        if our_moves == their_moves { prop_assert_eq!(score, 0); }
    }
}

// Feature: chess-engine-core, Property 27: Asymmetry Addict Scoring
// **Validates: Requirements 16.2, 16.3**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    fn property_27_asymmetry_addict_symmetry_penalty(_dummy in 0u32..100) {
        let a = AsymmetryAddict::new();
        let ctx = GameContext::new();
        // Starting position: symmetric pawns
        let sym = a.evaluate(&Board::new(), &ctx);
        // Position with only white pawns (asymmetric)
        let asym_board = Board::from_fen("4k3/8/8/8/8/8/P7/4K3 w - - 0 1").unwrap();
        let asym = a.evaluate(&asym_board, &ctx);
        prop_assert!(sym < asym, "sym={} should be < asym={}", sym, asym);
    }
    #[test]
    fn property_27_asymmetry_addict_imbalance_bonus(_dummy in 0u32..100) {
        let a = AsymmetryAddict::new();
        let ctx = GameContext::new();
        let imb = Board::from_fen("4k3/8/2n2n2/8/8/2B2B2/8/4K3 w - - 0 1").unwrap();
        let bal = Board::from_fen("4k3/8/2n2n2/8/8/2N2N2/8/4K3 w - - 0 1").unwrap();
        prop_assert!(a.evaluate(&imb, &ctx) >= a.evaluate(&bal, &ctx));
    }
}

// Feature: chess-engine-core, Property 28: Momentum Tracker Trend Alignment
// **Validates: Requirements 17.2, 17.3, 17.4**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    fn property_28_momentum_tracker_trend_alignment(
        base in -500i32..500i32, slope in 1i32..50i32, num_entries in 2u8..8u8,
    ) {
        let mt = MomentumTracker::new();
        let board = Board::new();
        let mut ctx_pos = GameContext::new();
        for i in 0..num_entries { ctx_pos.push_eval(base + slope * i as i32); }
        let sp = mt.evaluate(&board, &ctx_pos);
        if ctx_pos.momentum() > 0 { prop_assert!(sp > 0, "pos: m={}, s={}", ctx_pos.momentum(), sp); }
        let mut ctx_neg = GameContext::new();
        for i in 0..num_entries { ctx_neg.push_eval(base - slope * i as i32); }
        let sn = mt.evaluate(&board, &ctx_neg);
        if ctx_neg.momentum() < 0 { prop_assert!(sn < 0, "neg: m={}, s={}", ctx_neg.momentum(), sn); }
        let mut ctx_flat = GameContext::new();
        for _ in 0..num_entries { ctx_flat.push_eval(base); }
        prop_assert_eq!(mt.evaluate(&board, &ctx_flat), 0);
    }
}

// Feature: chess-engine-core, Property 29: Zugzwang Hunter Inverse Proportionality
// **Validates: Requirements 18.2, 18.3**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    fn property_29_zugzwang_hunter_inverse_proportionality(
        their_a in 1u32..30, extra in 1u32..30,
    ) {
        let z = ZugzwangHunter::new();
        let board = Board::new();
        let mut ca = GameContext::new();
        ca.opponent_moves = their_a;
        let mut cb = GameContext::new();
        cb.opponent_moves = their_a + extra;
        prop_assert!(z.evaluate(&board, &ca) >= z.evaluate(&board, &cb));
    }
    #[test]
    fn property_29_zugzwang_hunter_endgame_weight(their_moves in 1u32..30) {
        let z = ZugzwangHunter::new();
        let board = Board::new();
        let mut cn = GameContext::new();
        cn.opponent_moves = their_moves;
        cn.phase = GamePhase::Opening;
        let mut ce = GameContext::new();
        ce.opponent_moves = their_moves;
        ce.phase = GamePhase::Endgame;
        prop_assert!(z.evaluate(&board, &ce) > z.evaluate(&board, &cn));
    }
}
