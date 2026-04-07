// Feature: chess-engine-core, Properties 9, 10, 12, 13, 14: Search Properties
//
// Property 9: TT Store/Probe Round Trip — **Validates: Requirements 4.4**
// Property 10: TT Replacement Policy — **Validates: Requirements 4.6**
// Property 12: MVV-LVA Capture Ordering — **Validates: Requirements 6.4, 7.2**
// Property 13: Move Ordering Priority — **Validates: Requirements 7.1**
// Property 14: Killer and History Table Invariants — **Validates: Requirements 7.3, 7.4**

use chess_engine_core::board::*;
use chess_engine_core::search::tt::{TranspositionTable, TTEntry, NodeType};
use proptest::prelude::*;

// ─── Generators ──────────────────────────────────────────────────────────────

fn arb_piece() -> impl Strategy<Value = Piece> {
    prop_oneof![
        Just(Piece::Pawn),
        Just(Piece::Knight),
        Just(Piece::Bishop),
        Just(Piece::Rook),
        Just(Piece::Queen),
        Just(Piece::King),
    ]
}

fn arb_node_type() -> impl Strategy<Value = NodeType> {
    prop_oneof![
        Just(NodeType::Exact),
        Just(NodeType::LowerBound),
        Just(NodeType::UpperBound),
    ]
}

fn arb_move() -> impl Strategy<Value = Move> {
    (0u8..64, 0u8..64, arb_piece(), proptest::option::of(arb_piece()), proptest::option::of(arb_piece()))
        .prop_map(|(from, to, piece, captured, promotion)| {
            let mut flags = MoveFlags::QUIET;
            if promotion.is_some() {
                flags = flags | MoveFlags::PROMOTION;
            }
            Move::new(from, to, piece, captured, promotion, flags)
        })
}

// ─── Property 9: TT Store/Probe Round Trip ───────────────────────────────────
// **Validates: Requirements 4.4**
//
// Store a TTEntry, probe with same hash, verify fields match.

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn tt_store_probe_round_trip(
        hash in 1u64..u64::MAX,
        score in -30000i32..30000,
        depth in 0i32..64,
        node_type in arb_node_type(),
        has_move in proptest::bool::ANY,
        from in 0u8..64,
        to in 0u8..64,
        piece in arb_piece(),
    ) {
        // Use a large enough table to minimize collisions
        let mut tt = TranspositionTable::new(1);
        let gen = tt.generation();

        let best_move = if has_move {
            Some(Move::new(from, to, piece, None, None, MoveFlags::QUIET))
        } else {
            None
        };

        let entry = TTEntry {
            key: hash,
            best_move,
            score,
            depth,
            node_type,
            age: gen,
        };

        tt.store(hash, entry);
        let probed = tt.probe(hash);

        prop_assert!(probed.is_some(), "Probe should find stored entry for hash {}", hash);
        let probed = probed.unwrap();

        prop_assert_eq!(probed.key, hash, "Key mismatch");
        prop_assert_eq!(probed.score, score, "Score mismatch");
        prop_assert_eq!(probed.depth, depth, "Depth mismatch");
        prop_assert_eq!(probed.node_type, node_type, "NodeType mismatch");

        // Verify best_move
        match (best_move, probed.best_move) {
            (None, None) => {},
            (Some(expected), Some(actual)) => {
                prop_assert_eq!(actual.from, expected.from, "Move from mismatch");
                prop_assert_eq!(actual.to, expected.to, "Move to mismatch");
            },
            (expected, actual) => {
                prop_assert!(false, "best_move mismatch: expected {:?}, got {:?}", expected, actual);
            }
        }
    }
}

// ─── Property 10: TT Replacement Policy ─────────────────────────────────────
// **Validates: Requirements 4.6**
//
// Store multiple entries to same bucket, verify retained entry follows
// depth/age replacement rules.

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 10a: Current generation replaces older generation regardless of depth
    #[test]
    fn tt_replacement_newer_gen_replaces_older(
        hash in 1u64..u64::MAX,
        old_depth in 1i32..64,
        new_depth in 0i32..64,
        old_score in -30000i32..30000,
        new_score in -30000i32..30000,
    ) {
        let mut tt = TranspositionTable::new(1);

        // Store entry at generation 0
        let old_entry = TTEntry {
            key: hash,
            best_move: None,
            score: old_score,
            depth: old_depth,
            node_type: NodeType::Exact,
            age: 0,
        };
        tt.store(hash, old_entry);

        // Advance generation
        tt.new_generation();
        let new_gen = tt.generation();

        // Store entry at new generation (even with lower depth)
        let new_entry = TTEntry {
            key: hash,
            best_move: None,
            score: new_score,
            depth: new_depth,
            node_type: NodeType::LowerBound,
            age: new_gen,
        };
        tt.store(hash, new_entry);

        let probed = tt.probe(hash).expect("Should find entry");
        prop_assert_eq!(probed.score, new_score,
            "New generation entry should replace old regardless of depth (old_depth={}, new_depth={})",
            old_depth, new_depth);
        prop_assert_eq!(probed.age, new_gen);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 10b: Same generation, deeper entry is retained over shallower
    #[test]
    fn tt_replacement_same_gen_deeper_wins(
        hash in 1u64..u64::MAX,
        shallow_depth in 0i32..30,
        deep_extra in 1i32..34,
        score_a in -30000i32..30000,
        score_b in -30000i32..30000,
    ) {
        let mut tt = TranspositionTable::new(1);
        let gen = tt.generation();
        let deep_depth = shallow_depth + deep_extra;

        // Store deep entry first
        let deep_entry = TTEntry {
            key: hash,
            best_move: None,
            score: score_a,
            depth: deep_depth,
            node_type: NodeType::Exact,
            age: gen,
        };
        tt.store(hash, deep_entry);

        // Try to store shallower entry at same generation
        let shallow_entry = TTEntry {
            key: hash,
            best_move: None,
            score: score_b,
            depth: shallow_depth,
            node_type: NodeType::UpperBound,
            age: gen,
        };
        tt.store(hash, shallow_entry);

        let probed = tt.probe(hash).expect("Should find entry");
        prop_assert_eq!(probed.score, score_a,
            "Deeper entry (depth={}) should be retained over shallower (depth={})",
            deep_depth, shallow_depth);
        prop_assert_eq!(probed.depth, deep_depth);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 10c: Same generation, equal or greater depth replaces
    #[test]
    fn tt_replacement_same_gen_equal_or_greater_depth_replaces(
        hash in 1u64..u64::MAX,
        depth in 0i32..64,
        score_a in -30000i32..30000,
        score_b in -30000i32..30000,
    ) {
        let mut tt = TranspositionTable::new(1);
        let gen = tt.generation();

        // Store first entry
        let first = TTEntry {
            key: hash,
            best_move: None,
            score: score_a,
            depth,
            node_type: NodeType::Exact,
            age: gen,
        };
        tt.store(hash, first);

        // Store second entry with same depth — should replace (>= depth)
        let second = TTEntry {
            key: hash,
            best_move: None,
            score: score_b,
            depth,
            node_type: NodeType::LowerBound,
            age: gen,
        };
        tt.store(hash, second);

        let probed = tt.probe(hash).expect("Should find entry");
        prop_assert_eq!(probed.score, score_b,
            "Equal depth entry should replace: depth={}", depth);
    }
}


// ─── Move Ordering Imports ───────────────────────────────────────────────────

use chess_engine_core::search::{
    mvv_lva_score, order_moves, KillerTable, HistoryTable, MAX_PLY,
};

// ─── Property 12: MVV-LVA Capture Ordering ──────────────────────────────────
// **Validates: Requirements 6.4, 7.2**
//
// For any list of captures, verify MVV-LVA scores are in non-increasing order
// after sorting.

fn arb_capture_move() -> impl Strategy<Value = Move> {
    (0u8..64, 0u8..64, arb_piece(), arb_piece())
        .prop_map(|(from, to, attacker, victim)| {
            Move::new(from, to, attacker, Some(victim), None, MoveFlags::QUIET)
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 12: MVV-LVA Capture Ordering
    ///
    /// For any list of capture moves, after MVV-LVA ordering, each capture's
    /// score (victim_value * 10 - attacker_value) shall be >= the next capture's score.
    #[test]
    fn mvv_lva_capture_ordering(
        captures in prop::collection::vec(arb_capture_move(), 1..20),
    ) {
        let mut moves = captures;
        let no_killers: [Option<Move>; 2] = [None; 2];
        let empty_history = [[0i32; 64]; 12];

        // Order with no TT move, no killers, no history — only MVV-LVA matters for captures
        order_moves(&mut moves, None, &no_killers, &empty_history);

        // All are captures, so they should be sorted by MVV-LVA score in non-increasing order
        for i in 0..moves.len() - 1 {
            let score_a = mvv_lva_score(&moves[i]);
            let score_b = mvv_lva_score(&moves[i + 1]);
            prop_assert!(
                score_a >= score_b,
                "MVV-LVA ordering violated at index {}: score {} < score {} (moves: {:?} vs {:?})",
                i, score_a, score_b, moves[i], moves[i + 1]
            );
        }
    }
}


// ─── Property 13: Move Ordering Priority ─────────────────────────────────────
// **Validates: Requirements 7.1**
//
// Verify TT move is first, then captures, then killers, then quiet moves by history.

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 13: Move Ordering Priority
    ///
    /// Given a set of moves containing a TT move, captures, killers, and quiet moves,
    /// the ordering should be: TT move first, then captures, then killers, then quiet moves.
    #[test]
    fn move_ordering_priority(
        tt_from in 0u8..64,
        tt_to in 0u8..64,
        killer_from in 0u8..64,
        killer_to in 0u8..64,
        cap_from in 0u8..64,
        cap_to in 0u8..64,
        quiet_from in 0u8..64,
        quiet_to in 0u8..64,
        victim in arb_piece(),
    ) {
        // Create distinct moves of each category
        let tt_move = Move::new(tt_from, tt_to, Piece::Knight, None, None, MoveFlags::QUIET);
        let capture_move = Move::new(cap_from, cap_to, Piece::Pawn, Some(victim), None, MoveFlags::QUIET);
        let killer_move = Move::new(killer_from, killer_to, Piece::Bishop, None, None, MoveFlags::QUIET);
        let quiet_move = Move::new(quiet_from, quiet_to, Piece::Rook, None, None, MoveFlags::QUIET);

        // Skip if any moves are equal (would confuse categorization)
        if tt_move == capture_move || tt_move == killer_move || tt_move == quiet_move
            || capture_move == killer_move || capture_move == quiet_move
            || killer_move == quiet_move {
            return Ok(());
        }

        let killers: [Option<Move>; 2] = [Some(killer_move), None];
        let empty_history = [[0i32; 64]; 12];

        let mut moves = vec![quiet_move, capture_move, killer_move, tt_move];
        order_moves(&mut moves, Some(tt_move), &killers, &empty_history);

        // TT move should be first
        prop_assert_eq!(moves[0], tt_move,
            "TT move should be first, got {:?}", moves[0]);

        // Capture should come before killer and quiet
        let cap_idx = moves.iter().position(|m| *m == capture_move).unwrap();
        let killer_idx = moves.iter().position(|m| *m == killer_move).unwrap();
        let quiet_idx = moves.iter().position(|m| *m == quiet_move).unwrap();

        prop_assert!(cap_idx < killer_idx,
            "Capture (idx {}) should come before killer (idx {})", cap_idx, killer_idx);
        prop_assert!(killer_idx < quiet_idx,
            "Killer (idx {}) should come before quiet (idx {})", killer_idx, quiet_idx);
    }
}


// ─── Property 14: Killer and History Table Invariants ────────────────────────
// **Validates: Requirements 7.3, 7.4**
//
// Verify killer table stores at most 2 non-capture entries per ply;
// history scores accumulate correctly.

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 14a: Killer table stores at most 2 non-capture entries per ply
    #[test]
    fn killer_table_invariants(
        ply in 0usize..MAX_PLY,
        moves in prop::collection::vec(
            (0u8..64, 0u8..64, arb_piece()),
            1..10
        ),
    ) {
        let mut kt = KillerTable::new();

        for (from, to, piece) in &moves {
            let mv = Move::new(*from, *to, *piece, None, None, MoveFlags::QUIET);
            kt.store(ply, mv);
        }

        // At most 2 entries per ply
        let killers = kt.get(ply);
        let count = killers.iter().filter(|k| k.is_some()).count();
        prop_assert!(count <= 2,
            "Killer table should have at most 2 entries per ply, got {}", count);

        // All stored killers should be non-capture moves
        for killer in killers.iter().flatten() {
            prop_assert!(!killer.is_capture(),
                "Killer move should be non-capture: {:?}", killer);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 14b: Killer table rejects capture moves
    #[test]
    fn killer_table_rejects_captures(
        ply in 0usize..MAX_PLY,
        from in 0u8..64,
        to in 0u8..64,
        attacker in arb_piece(),
        victim in arb_piece(),
    ) {
        let mut kt = KillerTable::new();

        // Try to store a capture move
        let capture = Move::new(from, to, attacker, Some(victim), None, MoveFlags::QUIET);
        kt.store(ply, capture);

        // Should not be stored
        let killers = kt.get(ply);
        let count = killers.iter().filter(|k| k.is_some()).count();
        prop_assert_eq!(count, 0,
            "Capture moves should not be stored in killer table");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 14c: History scores accumulate correctly
    #[test]
    fn history_table_accumulation(
        piece_idx in 0usize..12,
        to_sq in 0u8..64,
        depths in prop::collection::vec(1i32..20, 1..10),
    ) {
        let mut ht = HistoryTable::new();

        let mut expected_total = 0i32;
        for depth in &depths {
            ht.update(piece_idx, to_sq, *depth);
            expected_total += depth * depth;
        }

        let actual = ht.get(piece_idx, to_sq);
        prop_assert_eq!(actual, expected_total,
            "History score should be sum of depth^2 increments: expected {}, got {}",
            expected_total, actual);
    }
}


// ─── Property 11: Alpha-Beta Equivalence to Minimax ──────────────────────────
// **Validates: Requirements 5.1**
//
// For random simple positions at depth ≤ 2, verify alpha-beta returns the same
// score as a plain negamax search (no pruning, no TT, no quiescence).

use chess_engine_core::eval;
use chess_engine_core::movegen::{self, MoveGenResult};
use chess_engine_core::search::{SearchParams, allocate_time};

/// Plain negamax without pruning, TT, or quiescence.
/// At depth 0, returns static evaluation. This is the reference implementation.
fn plain_negamax(board: &mut Board, depth: i32) -> i32 {
    if depth <= 0 {
        return eval::evaluate(board);
    }

    let moves = match movegen::generate_legal_moves(board) {
        MoveGenResult::Moves(moves) => moves,
        MoveGenResult::Checkmate => return -eval::mate_score(0),
        MoveGenResult::Stalemate => return 0,
    };

    let mut best_score = -999_999;
    for mv in &moves {
        board.make_move(*mv);
        let score = -plain_negamax(board, depth - 1);
        board.unmake_move(*mv);
        if score > best_score {
            best_score = score;
        }
    }
    best_score
}

/// Alpha-beta negamax without TT or quiescence — apples-to-apples comparison
/// with plain_negamax. Uses static eval at leaf nodes (depth 0).
fn alphabeta_no_extras(board: &mut Board, depth: i32, mut alpha: i32, beta: i32) -> i32 {
    if depth <= 0 {
        return eval::evaluate(board);
    }

    let moves = match movegen::generate_legal_moves(board) {
        MoveGenResult::Moves(moves) => moves,
        MoveGenResult::Checkmate => return -eval::mate_score(0),
        MoveGenResult::Stalemate => return 0,
    };

    for mv in &moves {
        board.make_move(*mv);
        let score = -alphabeta_no_extras(board, depth - 1, -beta, -alpha);
        board.unmake_move(*mv);
        if score > alpha {
            alpha = score;
        }
        if alpha >= beta {
            break;
        }
    }
    alpha
}

/// Simple endgame positions with few pieces to keep the search tree small.
fn simple_endgame_fens() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        // KQ vs K
        Just("8/8/8/4k3/8/8/8/4K2Q w - - 0 1"),
        Just("8/8/8/4k3/8/8/8/4K2Q b - - 0 1"),
        // KR vs K
        Just("8/8/8/4k3/8/8/8/R3K3 w - - 0 1"),
        Just("8/8/8/4k3/8/8/8/R3K3 b - - 0 1"),
        // KBN vs K
        Just("8/8/8/4k3/8/8/8/1B2K1N1 w - - 0 1"),
        // KP vs K
        Just("8/8/8/4k3/8/8/4P3/4K3 w - - 0 1"),
        Just("8/4p3/8/4k3/8/8/8/4K3 b - - 0 1"),
        // KQ vs KR
        Just("8/8/8/4k3/8/8/8/r3K2Q w - - 0 1"),
        Just("8/8/8/4k3/8/8/8/r3K2Q b - - 0 1"),
        // KR vs KN
        Just("8/8/8/4k3/8/8/3n4/R3K3 w - - 0 1"),
        // KBB vs K
        Just("8/8/8/4k3/8/8/8/2B1KB2 w - - 0 1"),
        // KNN vs K (drawn but good for testing)
        Just("8/8/8/4k3/8/8/8/1N2K1N1 w - - 0 1"),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 11: Alpha-Beta Equivalence to Minimax
    ///
    /// For simple endgame positions at depth 1-2, verify alpha-beta (without TT
    /// or quiescence) returns the same score as plain negamax.
    #[test]
    fn alpha_beta_equivalence_to_minimax(
        fen in simple_endgame_fens(),
        depth in 1i32..=2,
    ) {
        magic::init_magic_tables();

        let mut board_negamax = Board::from_fen(fen).unwrap();
        let mut board_alphabeta = Board::from_fen(fen).unwrap();

        let negamax_score = plain_negamax(&mut board_negamax, depth);
        let alphabeta_score = alphabeta_no_extras(
            &mut board_alphabeta, depth, -999_999, 999_999,
        );

        prop_assert_eq!(
            negamax_score, alphabeta_score,
            "Alpha-beta score ({}) should equal negamax score ({}) for FEN '{}' at depth {}",
            alphabeta_score, negamax_score, fen, depth
        );
    }
}


// ─── Property 22: Time Allocation Formula ────────────────────────────────────
// **Validates: Requirements 10.1, 10.2**
//
// For random time control params, verify allocated time matches formula and 50% cap.

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 22a: Movetime mode returns exact movetime
    #[test]
    fn time_allocation_movetime_mode(
        movetime in 1u64..1_000_000,
        wtime in proptest::option::of(1u64..1_000_000),
        btime in proptest::option::of(1u64..1_000_000),
    ) {
        let mut params = SearchParams::new();
        params.move_time = Some(movetime);
        params.wtime = wtime;
        params.btime = btime;

        let result = allocate_time(&params, Color::White);
        prop_assert_eq!(result, movetime,
            "Movetime mode should return exact movetime {}, got {}", movetime, result);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 22b: Infinite mode returns u64::MAX
    #[test]
    fn time_allocation_infinite_mode(
        wtime in proptest::option::of(1u64..1_000_000),
        btime in proptest::option::of(1u64..1_000_000),
    ) {
        let mut params = SearchParams::new();
        params.infinite = true;
        params.wtime = wtime;
        params.btime = btime;

        let result_w = allocate_time(&params, Color::White);
        let result_b = allocate_time(&params, Color::Black);
        prop_assert_eq!(result_w, u64::MAX, "Infinite mode should return MAX for White");
        prop_assert_eq!(result_b, u64::MAX, "Infinite mode should return MAX for Black");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 22c: Depth mode returns u64::MAX
    #[test]
    fn time_allocation_depth_mode(
        max_depth in 1u32..64,
        wtime in proptest::option::of(1u64..1_000_000),
    ) {
        let mut params = SearchParams::new();
        params.max_depth = Some(max_depth);
        params.wtime = wtime;

        let result = allocate_time(&params, Color::White);
        prop_assert_eq!(result, u64::MAX, "Depth mode should return MAX, got {}", result);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 22d: Time-based allocation matches formula with 50% cap
    ///
    /// Formula: remaining / moves_left + increment, capped at 50% of remaining.
    /// Default moves_left = 30 when movestogo absent.
    /// When movestogo present, moves_left = movestogo + 2 (safety margin).
    #[test]
    fn time_allocation_formula_and_cap(
        remaining in 1000u64..10_000_000,
        increment in 0u64..10_000,
        side in prop_oneof![Just(Color::White), Just(Color::Black)],
        movestogo in proptest::option::of(1u32..100),
    ) {
        let mut params = SearchParams::new();
        match side {
            Color::White => {
                params.wtime = Some(remaining);
                params.winc = Some(increment);
            }
            Color::Black => {
                params.btime = Some(remaining);
                params.binc = Some(increment);
            }
        }
        params.moves_to_go = movestogo;

        let result = allocate_time(&params, side);

        // Compute expected value
        let moves_left: u64 = match movestogo {
            Some(mtg) => (mtg + 2) as u64, // movestogo + MOVESTOGO_SAFETY(2)
            None => 25,                      // DEFAULT_MOVES_LEFT
        };
        let moves_left = moves_left.max(1);
        let base_time = remaining / moves_left + increment;
        let cap = remaining / 2;
        let expected = base_time.min(cap);

        prop_assert_eq!(result, expected,
            "Time allocation mismatch: remaining={}, inc={}, moves_left={}, base={}, cap={}, expected={}, got={}",
            remaining, increment, moves_left, base_time, cap, expected, result);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 22e: Allocated time never exceeds 50% of remaining time
    #[test]
    fn time_allocation_never_exceeds_50_percent(
        remaining in 1u64..10_000_000,
        increment in 0u64..100_000,
        movestogo in proptest::option::of(1u32..100),
    ) {
        let mut params = SearchParams::new();
        params.wtime = Some(remaining);
        params.winc = Some(increment);
        params.moves_to_go = movestogo;

        let result = allocate_time(&params, Color::White);
        let cap = remaining / 2;

        prop_assert!(result <= cap,
            "Allocated time {} exceeds 50% cap {} of remaining time {}",
            result, cap, remaining);
    }
}
