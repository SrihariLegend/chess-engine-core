// Legal move generation and perft

use crate::board::magic;
use crate::board::{Board, Color, Move, MoveFlags, Piece};

/// Result of legal move generation.
#[derive(Debug)]
pub enum MoveGenResult {
    Moves(Vec<Move>),
    Checkmate,
    Stalemate,
}

/// Generates all legal moves for the side to move.
///
/// Returns `MoveGenResult::Checkmate` if no legal moves and king is in check,
/// `MoveGenResult::Stalemate` if no legal moves and king is not in check,
/// or `MoveGenResult::Moves(vec)` with all legal moves.
pub fn generate_legal_moves(board: &mut Board) -> MoveGenResult {
    let pseudo = generate_pseudo_legal_moves(board);
    let us = board.side_to_move;

    let mut legal = Vec::new();
    for mv in pseudo {
        board.make_move(mv);
        if !board.is_in_check(us) {
            legal.push(mv);
        }
        board.unmake_move(mv);
    }

    if legal.is_empty() {
        if board.is_in_check(us) {
            MoveGenResult::Checkmate
        } else {
            MoveGenResult::Stalemate
        }
    } else {
        MoveGenResult::Moves(legal)
    }
}

/// Generates all pseudo-legal moves (may leave own king in check).
fn generate_pseudo_legal_moves(board: &Board) -> Vec<Move> {
    let mut moves = Vec::with_capacity(64);
    let us = board.side_to_move;
    let us_idx = us.index();
    let them = us.opposite();
    let them_idx = them.index();
    let our_occ = board.occupancy[us_idx];
    let their_occ = board.occupancy[them_idx];
    let all_occ = board.all_occupancy;

    // Pawns
    generate_pawn_moves(board, us, our_occ, their_occ, all_occ, &mut moves);

    // Knights
    let mut knights = board.pieces[us_idx][Piece::Knight.index()];
    while knights != 0 {
        let from = knights.trailing_zeros() as u8;
        let attacks = magic::knight_attacks(from) & !our_occ;
        add_moves_from_bb(from, attacks, their_occ, Piece::Knight, board, them, &mut moves);
        knights &= knights - 1;
    }

    // Bishops
    let mut bishops = board.pieces[us_idx][Piece::Bishop.index()];
    while bishops != 0 {
        let from = bishops.trailing_zeros() as u8;
        let attacks = magic::bishop_attacks(from, all_occ) & !our_occ;
        add_moves_from_bb(from, attacks, their_occ, Piece::Bishop, board, them, &mut moves);
        bishops &= bishops - 1;
    }

    // Rooks
    let mut rooks = board.pieces[us_idx][Piece::Rook.index()];
    while rooks != 0 {
        let from = rooks.trailing_zeros() as u8;
        let attacks = magic::rook_attacks(from, all_occ) & !our_occ;
        add_moves_from_bb(from, attacks, their_occ, Piece::Rook, board, them, &mut moves);
        rooks &= rooks - 1;
    }

    // Queens
    let mut queens = board.pieces[us_idx][Piece::Queen.index()];
    while queens != 0 {
        let from = queens.trailing_zeros() as u8;
        let attacks = magic::queen_attacks(from, all_occ) & !our_occ;
        add_moves_from_bb(from, attacks, their_occ, Piece::Queen, board, them, &mut moves);
        queens &= queens - 1;
    }

    // King (normal moves)
    let king_bb = board.pieces[us_idx][Piece::King.index()];
    if king_bb != 0 {
        let from = king_bb.trailing_zeros() as u8;
        let attacks = magic::king_attacks(from) & !our_occ;
        add_moves_from_bb(from, attacks, their_occ, Piece::King, board, them, &mut moves);
    }

    // Castling
    generate_castling_moves(board, us, all_occ, &mut moves);

    moves
}

/// Helper: add moves from an attack bitboard, distinguishing captures from quiet moves.
fn add_moves_from_bb(
    from: u8,
    mut targets: u64,
    their_occ: u64,
    piece: Piece,
    board: &Board,
    them: Color,
    moves: &mut Vec<Move>,
) {
    while targets != 0 {
        let to = targets.trailing_zeros() as u8;
        let to_bit = 1u64 << to;
        if their_occ & to_bit != 0 {
            // Capture
            let captured = find_captured_piece(board, them, to);
            moves.push(Move::new(from, to, piece, Some(captured), None, MoveFlags::QUIET));
        } else {
            moves.push(Move::quiet(from, to, piece));
        }
        targets &= targets - 1;
    }
}

/// Find which piece type is on a given square for the given color.
fn find_captured_piece(board: &Board, color: Color, sq: u8) -> Piece {
    let mask = 1u64 << sq;
    let idx = color.index();
    if board.pieces[idx][Piece::Pawn.index()] & mask != 0 { return Piece::Pawn; }
    if board.pieces[idx][Piece::Knight.index()] & mask != 0 { return Piece::Knight; }
    if board.pieces[idx][Piece::Bishop.index()] & mask != 0 { return Piece::Bishop; }
    if board.pieces[idx][Piece::Rook.index()] & mask != 0 { return Piece::Rook; }
    if board.pieces[idx][Piece::Queen.index()] & mask != 0 { return Piece::Queen; }
    // Should not happen in valid positions, but king can be "captured" in pseudo-legal
    Piece::King
}

/// Generate all pawn moves (pushes, double pushes, captures, en passant, promotions).
fn generate_pawn_moves(
    board: &Board,
    us: Color,
    _our_occ: u64,
    their_occ: u64,
    all_occ: u64,
    moves: &mut Vec<Move>,
) {
    let us_idx = us.index();
    let them = us.opposite();
    let pawns = board.pieces[us_idx][Piece::Pawn.index()];

    let (push_dir, start_rank, promo_rank): (i8, u8, u8) = match us {
        Color::White => (8, 1, 7),
        Color::Black => (-8, 6, 0),
    };

    let mut bb = pawns;
    while bb != 0 {
        let from = bb.trailing_zeros() as u8;
        let rank = from / 8;
        let _file = from % 8;

        let to_sq = (from as i8 + push_dir) as u8;

        // Single push
        if all_occ & (1u64 << to_sq) == 0 {
            if rank == (if us == Color::White { promo_rank - 1 } else { promo_rank + 1 }) {
                // Promotion
                add_promotion_moves(from, to_sq, None, moves);
            } else {
                moves.push(Move::quiet(from, to_sq, Piece::Pawn));

                // Double push
                if rank == start_rank {
                    let double_to = (to_sq as i8 + push_dir) as u8;
                    if all_occ & (1u64 << double_to) == 0 {
                        moves.push(Move::new(
                            from, double_to, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH,
                        ));
                    }
                }
            }
        }

        // Captures
        let pawn_attacks = magic::pawn_attacks(from, us);
        let mut captures = pawn_attacks & their_occ;
        while captures != 0 {
            let cap_sq = captures.trailing_zeros() as u8;
            let captured = find_captured_piece(board, them, cap_sq);
            let cap_rank = cap_sq / 8;
            if cap_rank == promo_rank {
                add_promotion_moves(from, cap_sq, Some(captured), moves);
            } else {
                moves.push(Move::new(from, cap_sq, Piece::Pawn, Some(captured), None, MoveFlags::QUIET));
            }
            captures &= captures - 1;
        }

        // En passant
        if let Some(ep_sq) = board.en_passant {
            if pawn_attacks & (1u64 << ep_sq) != 0 {
                moves.push(Move::new(
                    from, ep_sq, Piece::Pawn, Some(Piece::Pawn), None, MoveFlags::EN_PASSANT,
                ));
            }
        }

        bb &= bb - 1;
    }
}

/// Add 4 promotion moves (Q, R, B, N) for a pawn reaching the promotion rank.
fn add_promotion_moves(from: u8, to: u8, captured: Option<Piece>, moves: &mut Vec<Move>) {
    for promo_piece in [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight] {
        moves.push(Move::new(
            from, to, Piece::Pawn, captured, Some(promo_piece), MoveFlags::PROMOTION,
        ));
    }
}

/// Generate castling moves if legal (pseudo-legal: doesn't check if king ends in check,
/// but does check path is clear and king doesn't pass through attacked squares).
fn generate_castling_moves(board: &Board, us: Color, all_occ: u64, moves: &mut Vec<Move>) {
    let them = us.opposite();

    match us {
        Color::White => {
            // Kingside: e1-g1, rook h1-f1
            if board.castling.white_kingside() {
                // Squares f1(5) and g1(6) must be empty
                if all_occ & ((1u64 << 5) | (1u64 << 6)) == 0 {
                    // King must not be in check, and must not pass through f1 attacked
                    if !board.is_square_attacked(4, them) && !board.is_square_attacked(5, them) && !board.is_square_attacked(6, them) {
                        moves.push(Move::new(4, 6, Piece::King, None, None, MoveFlags::KING_CASTLE));
                    }
                }
            }
            // Queenside: e1-c1, rook a1-d1
            if board.castling.white_queenside() {
                // Squares b1(1), c1(2), d1(3) must be empty
                if all_occ & ((1u64 << 1) | (1u64 << 2) | (1u64 << 3)) == 0 {
                    if !board.is_square_attacked(4, them) && !board.is_square_attacked(3, them) && !board.is_square_attacked(2, them) {
                        moves.push(Move::new(4, 2, Piece::King, None, None, MoveFlags::QUEEN_CASTLE));
                    }
                }
            }
        }
        Color::Black => {
            // Kingside: e8-g8, rook h8-f8
            if board.castling.black_kingside() {
                if all_occ & ((1u64 << 61) | (1u64 << 62)) == 0 {
                    if !board.is_square_attacked(60, them) && !board.is_square_attacked(61, them) && !board.is_square_attacked(62, them) {
                        moves.push(Move::new(60, 62, Piece::King, None, None, MoveFlags::KING_CASTLE));
                    }
                }
            }
            // Queenside: e8-c8, rook a8-d8
            if board.castling.black_queenside() {
                if all_occ & ((1u64 << 57) | (1u64 << 58) | (1u64 << 59)) == 0 {
                    if !board.is_square_attacked(60, them) && !board.is_square_attacked(59, them) && !board.is_square_attacked(58, them) {
                        moves.push(Move::new(60, 58, Piece::King, None, None, MoveFlags::QUEEN_CASTLE));
                    }
                }
            }
        }
    }
}

/// Generates all legal capture moves and queen promotions for the side to move.
///
/// Used by quiescence search. Includes:
/// - All captures (including en passant)
/// - Queen promotions (even non-capturing ones)
///
/// Requires `&mut Board` for make/unmake legality checking.
pub fn generate_captures(board: &mut Board) -> Vec<Move> {
    let pseudo = generate_pseudo_legal_captures(board);
    let us = board.side_to_move;

    let mut legal = Vec::new();
    for mv in pseudo {
        board.make_move(mv);
        if !board.is_in_check(us) {
            legal.push(mv);
        }
        board.unmake_move(mv);
    }

    legal
}

/// Counts the number of leaf nodes at the given depth (performance test).
///
/// Used to verify move generation correctness against published results.
/// At depth 0, returns 1. Otherwise generates all legal moves, makes each,
/// recursively counts at depth-1, and unmakes.
pub fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = match generate_legal_moves(board) {
        MoveGenResult::Moves(moves) => moves,
        MoveGenResult::Checkmate | MoveGenResult::Stalemate => return 0,
    };

    let mut nodes = 0u64;
    for mv in moves {
        board.make_move(mv);
        nodes += perft(board, depth - 1);
        board.unmake_move(mv);
    }
    nodes
}

/// Returns per-move node counts at the given depth (perft divide).
///
/// For each legal move at the root, makes the move, calls perft at depth-1,
/// and returns a vector of (move, node_count) pairs.
pub fn perft_divide(board: &mut Board, depth: u32) -> Vec<(Move, u64)> {
    let moves = match generate_legal_moves(board) {
        MoveGenResult::Moves(moves) => moves,
        MoveGenResult::Checkmate | MoveGenResult::Stalemate => return Vec::new(),
    };

    let mut results = Vec::with_capacity(moves.len());
    for mv in moves {
        board.make_move(mv);
        let count = if depth <= 1 { 1 } else { perft(board, depth - 1) };
        board.unmake_move(mv);
        results.push((mv, count));
    }
    results
}

/// Generates all legal moves when the side to move is in check.
///
/// Returns evasion moves: king moves, captures of the checking piece, and blocks.
/// If not in check, returns an empty vector.
///
/// Requires `&mut Board` for make/unmake legality checking.
pub fn generate_evasions(board: &mut Board) -> Vec<Move> {
    let us = board.side_to_move;
    if !board.is_in_check(us) {
        return Vec::new();
    }

    // When in check, all legal moves are evasions
    match generate_legal_moves(board) {
        MoveGenResult::Moves(moves) => moves,
        MoveGenResult::Checkmate => Vec::new(),
        MoveGenResult::Stalemate => Vec::new(),
    }
}

/// Generates pseudo-legal capture moves and queen promotions.
fn generate_pseudo_legal_captures(board: &Board) -> Vec<Move> {
    let mut moves = Vec::with_capacity(32);
    let us = board.side_to_move;
    let us_idx = us.index();
    let them = us.opposite();
    let them_idx = them.index();
    let our_occ = board.occupancy[us_idx];
    let their_occ = board.occupancy[them_idx];
    let all_occ = board.all_occupancy;

    // Pawn captures, en passant, and queen promotions
    generate_pawn_captures_and_queen_promos(board, us, their_occ, all_occ, &mut moves);

    // Knight captures
    let mut knights = board.pieces[us_idx][Piece::Knight.index()];
    while knights != 0 {
        let from = knights.trailing_zeros() as u8;
        let captures = magic::knight_attacks(from) & their_occ;
        add_capture_moves(from, captures, Piece::Knight, board, them, &mut moves);
        knights &= knights - 1;
    }

    // Bishop captures
    let mut bishops = board.pieces[us_idx][Piece::Bishop.index()];
    while bishops != 0 {
        let from = bishops.trailing_zeros() as u8;
        let captures = magic::bishop_attacks(from, all_occ) & their_occ;
        add_capture_moves(from, captures, Piece::Bishop, board, them, &mut moves);
        bishops &= bishops - 1;
    }

    // Rook captures
    let mut rooks = board.pieces[us_idx][Piece::Rook.index()];
    while rooks != 0 {
        let from = rooks.trailing_zeros() as u8;
        let captures = magic::rook_attacks(from, all_occ) & their_occ;
        add_capture_moves(from, captures, Piece::Rook, board, them, &mut moves);
        rooks &= rooks - 1;
    }

    // Queen captures
    let mut queens = board.pieces[us_idx][Piece::Queen.index()];
    while queens != 0 {
        let from = queens.trailing_zeros() as u8;
        let captures = magic::queen_attacks(from, all_occ) & their_occ;
        add_capture_moves(from, captures, Piece::Queen, board, them, &mut moves);
        queens &= queens - 1;
    }

    // King captures
    let king_bb = board.pieces[us_idx][Piece::King.index()];
    if king_bb != 0 {
        let from = king_bb.trailing_zeros() as u8;
        let captures = magic::king_attacks(from) & their_occ & !our_occ;
        add_capture_moves(from, captures, Piece::King, board, them, &mut moves);
    }

    moves
}

/// Helper: add only capture moves from a target bitboard.
fn add_capture_moves(
    from: u8,
    mut targets: u64,
    piece: Piece,
    board: &Board,
    them: Color,
    moves: &mut Vec<Move>,
) {
    while targets != 0 {
        let to = targets.trailing_zeros() as u8;
        let captured = find_captured_piece(board, them, to);
        moves.push(Move::new(from, to, piece, Some(captured), None, MoveFlags::QUIET));
        targets &= targets - 1;
    }
}

/// Generate pawn captures, en passant, and queen promotions (including non-capture queen promos).
fn generate_pawn_captures_and_queen_promos(
    board: &Board,
    us: Color,
    their_occ: u64,
    all_occ: u64,
    moves: &mut Vec<Move>,
) {
    let us_idx = us.index();
    let them = us.opposite();
    let pawns = board.pieces[us_idx][Piece::Pawn.index()];

    let (push_dir, promo_rank): (i8, u8) = match us {
        Color::White => (8, 7),
        Color::Black => (-8, 0),
    };

    let pre_promo_rank = match us {
        Color::White => promo_rank - 1, // rank 6
        Color::Black => promo_rank + 1, // rank 1
    };

    let mut bb = pawns;
    while bb != 0 {
        let from = bb.trailing_zeros() as u8;
        let rank = from / 8;

        // Pawn captures (including promotion captures)
        let pawn_attacks = magic::pawn_attacks(from, us);
        let mut captures = pawn_attacks & their_occ;
        while captures != 0 {
            let cap_sq = captures.trailing_zeros() as u8;
            let captured = find_captured_piece(board, them, cap_sq);
            let cap_rank = cap_sq / 8;
            if cap_rank == promo_rank {
                // Promotion capture — only queen promotion for quiescence
                moves.push(Move::new(
                    from, cap_sq, Piece::Pawn, Some(captured), Some(Piece::Queen), MoveFlags::PROMOTION,
                ));
            } else {
                // Normal capture
                moves.push(Move::new(from, cap_sq, Piece::Pawn, Some(captured), None, MoveFlags::QUIET));
            }
            captures &= captures - 1;
        }

        // En passant
        if let Some(ep_sq) = board.en_passant {
            if pawn_attacks & (1u64 << ep_sq) != 0 {
                moves.push(Move::new(
                    from, ep_sq, Piece::Pawn, Some(Piece::Pawn), None, MoveFlags::EN_PASSANT,
                ));
            }
        }

        // Non-capture queen promotions (pawn push to promotion rank)
        if rank == pre_promo_rank {
            let to_sq = (from as i8 + push_dir) as u8;
            if all_occ & (1u64 << to_sq) == 0 {
                moves.push(Move::new(
                    from, to_sq, Piece::Pawn, None, Some(Piece::Queen), MoveFlags::PROMOTION,
                ));
            }
        }

        bb &= bb - 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::magic::init_magic_tables;

    fn init() {
        init_magic_tables();
    }

    #[test]
    fn starting_position_has_20_legal_moves() {
        init();
        let mut board = Board::new();
        match generate_legal_moves(&mut board) {
            MoveGenResult::Moves(moves) => {
                assert_eq!(moves.len(), 20, "Starting position should have 20 legal moves, got {}", moves.len());
            }
            other => panic!("Expected Moves, got {:?}", other),
        }
    }

    #[test]
    fn fools_mate_is_checkmate() {
        init();
        // After 1.f3 e5 2.g4 Qh4# — Black delivers checkmate
        let mut board = Board::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3").unwrap();
        match generate_legal_moves(&mut board) {
            MoveGenResult::Checkmate => {} // expected
            other => panic!("Expected Checkmate, got {:?}", other),
        }
    }

    #[test]
    fn stalemate_position() {
        init();
        // Classic stalemate: Black king on a8, White king on c6, White queen on b6
        // Black to move, no legal moves, not in check
        let mut board = Board::from_fen("k7/8/1QK5/8/8/8/8/8 b - - 0 1").unwrap();
        match generate_legal_moves(&mut board) {
            MoveGenResult::Stalemate => {} // expected
            other => panic!("Expected Stalemate, got {:?}", other),
        }
    }

    #[test]
    fn en_passant_capture_generated() {
        init();
        // White pawn on e5, Black just played d7-d5, en passant on d6
        let mut board = Board::from_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3").unwrap();
        match generate_legal_moves(&mut board) {
            MoveGenResult::Moves(moves) => {
                let ep_moves: Vec<_> = moves.iter().filter(|m| m.is_en_passant()).collect();
                assert!(!ep_moves.is_empty(), "Should generate en passant capture");
                // e5 captures d6
                assert!(ep_moves.iter().any(|m| m.from == 36 && m.to == 43),
                    "Should have en passant from e5(36) to d6(43)");
            }
            other => panic!("Expected Moves, got {:?}", other),
        }
    }

    #[test]
    fn castling_moves_generated() {
        init();
        // Position where white can castle both sides
        let mut board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        match generate_legal_moves(&mut board) {
            MoveGenResult::Moves(moves) => {
                let castle_moves: Vec<_> = moves.iter().filter(|m| m.is_castling()).collect();
                assert_eq!(castle_moves.len(), 2, "Should generate 2 castling moves (kingside + queenside)");
                assert!(castle_moves.iter().any(|m| m.flags.contains(MoveFlags::KING_CASTLE)),
                    "Should have kingside castle");
                assert!(castle_moves.iter().any(|m| m.flags.contains(MoveFlags::QUEEN_CASTLE)),
                    "Should have queenside castle");
            }
            other => panic!("Expected Moves, got {:?}", other),
        }
    }

    #[test]
    fn in_check_only_generates_evasions() {
        init();
        // White king on e1 in check from black queen on e8, limited pieces
        // Simple check: Ke1 in check from Qe8 along e-file
        let mut board = Board::from_fen("4q3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert!(board.is_in_check(Color::White), "White should be in check");
        match generate_legal_moves(&mut board) {
            MoveGenResult::Moves(moves) => {
                // All moves should resolve the check
                for mv in &moves {
                    let mut test_board = Board::from_fen("4q3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
                    test_board.make_move(*mv);
                    assert!(!test_board.is_in_check(Color::White),
                        "Move {:?} should resolve check", mv);
                }
                // King can move to d1, d2, f1, f2 (not e2 which is still on the file)
                assert!(!moves.is_empty(), "Should have some evasion moves");
            }
            other => panic!("Expected Moves with evasions, got {:?}", other),
        }
    }

    #[test]
    fn promotion_generates_four_moves() {
        init();
        // White pawn on a7, about to promote
        let mut board = Board::from_fen("8/P3k3/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        match generate_legal_moves(&mut board) {
            MoveGenResult::Moves(moves) => {
                let promo_moves: Vec<_> = moves.iter().filter(|m| m.is_promotion()).collect();
                assert_eq!(promo_moves.len(), 4, "Should generate 4 promotion moves (Q/R/B/N)");
                assert!(promo_moves.iter().any(|m| m.promotion == Some(Piece::Queen)));
                assert!(promo_moves.iter().any(|m| m.promotion == Some(Piece::Rook)));
                assert!(promo_moves.iter().any(|m| m.promotion == Some(Piece::Bishop)));
                assert!(promo_moves.iter().any(|m| m.promotion == Some(Piece::Knight)));
            }
            other => panic!("Expected Moves, got {:?}", other),
        }
    }

    #[test]
    fn is_in_check_starting_position() {
        init();
        let board = Board::new();
        assert!(!board.is_in_check(Color::White));
        assert!(!board.is_in_check(Color::Black));
    }

    #[test]
    fn is_in_check_fools_mate() {
        init();
        let board = Board::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3").unwrap();
        assert!(board.is_in_check(Color::White));
    }

    // ── generate_captures tests ──────────────────────────────────────────

    #[test]
    fn generate_captures_starting_position_returns_empty() {
        init();
        let mut board = Board::new();
        let captures = generate_captures(&mut board);
        assert_eq!(captures.len(), 0, "Starting position has no captures");
    }

    #[test]
    fn generate_captures_returns_only_captures_and_queen_promos() {
        init();
        // Position with captures available: Italian Game after 1.e4 e5 2.Nf3 Nc6 3.Bc4
        // White knight on f3 can't capture anything, but let's use a position with actual captures
        // Use a position where white pawn on e5 can capture black pawn on d5
        let mut board = Board::from_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3").unwrap();
        let captures = generate_captures(&mut board);

        // All returned moves should be captures or queen promotions
        for mv in &captures {
            assert!(
                mv.is_capture() || (mv.is_promotion() && mv.promotion == Some(Piece::Queen)),
                "generate_captures returned a non-capture, non-queen-promotion move: {:?}", mv
            );
        }
        // Should include the en passant capture
        assert!(captures.iter().any(|m| m.is_en_passant()),
            "Should include en passant capture");
    }

    #[test]
    fn generate_captures_includes_queen_promotions_without_capture() {
        init();
        // White pawn on a7 about to promote, no piece to capture on a8
        let mut board = Board::from_fen("8/P3k3/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let captures = generate_captures(&mut board);

        // Should include queen promotion push (a7-a8=Q)
        let queen_promos: Vec<_> = captures.iter()
            .filter(|m| m.is_promotion() && m.promotion == Some(Piece::Queen) && !m.is_capture())
            .collect();
        assert!(!queen_promos.is_empty(),
            "generate_captures should include non-capture queen promotions");
        // Should NOT include underpromotions (rook, bishop, knight) for non-captures
        let underpromos: Vec<_> = captures.iter()
            .filter(|m| m.is_promotion() && m.promotion != Some(Piece::Queen) && !m.is_capture())
            .collect();
        assert!(underpromos.is_empty(),
            "generate_captures should not include non-capture underpromotions");
    }

    // ── generate_evasions tests ──────────────────────────────────────────

    #[test]
    fn generate_evasions_from_check_returns_evasion_moves() {
        init();
        // White king on e1 in check from black queen on e8
        let mut board = Board::from_fen("4q3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert!(board.is_in_check(Color::White));

        let evasions = generate_evasions(&mut board);
        assert!(!evasions.is_empty(), "Should have evasion moves when in check");

        // All evasion moves should resolve the check
        for mv in &evasions {
            let mut test_board = Board::from_fen("4q3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
            test_board.make_move(*mv);
            assert!(!test_board.is_in_check(Color::White),
                "Evasion move {:?} should resolve check", mv);
        }
    }

    #[test]
    fn generate_evasions_not_in_check_returns_empty() {
        init();
        // Starting position — not in check
        let mut board = Board::new();
        let evasions = generate_evasions(&mut board);
        assert!(evasions.is_empty(), "Should return empty when not in check");
    }

    #[test]
    fn generate_evasions_checkmate_returns_empty() {
        init();
        // Fool's mate — checkmate, no evasions possible
        let mut board = Board::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3").unwrap();
        assert!(board.is_in_check(Color::White));
        let evasions = generate_evasions(&mut board);
        assert!(evasions.is_empty(), "Checkmate position should have no evasions");
    }

    // ── perft tests ──────────────────────────────────────────────────────

    #[test]
    fn perft_initial_position_depth_1() {
        init();
        let mut board = Board::new();
        assert_eq!(perft(&mut board, 1), 20);
    }

    #[test]
    fn perft_initial_position_depth_2() {
        init();
        let mut board = Board::new();
        assert_eq!(perft(&mut board, 2), 400);
    }

    #[test]
    fn perft_initial_position_depth_3() {
        init();
        let mut board = Board::new();
        assert_eq!(perft(&mut board, 3), 8902);
    }

    #[test]
    fn perft_initial_position_depth_4() {
        init();
        let mut board = Board::new();
        assert_eq!(perft(&mut board, 4), 197281);
    }

    #[test]
    fn perft_kiwipete_depth_1() {
        init();
        // Kiwipete: r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -
        let mut board = Board::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"
        ).unwrap();
        assert_eq!(perft(&mut board, 1), 48);
    }

    #[test]
    fn perft_divide_initial_position_depth_1() {
        init();
        let mut board = Board::new();
        let results = perft_divide(&mut board, 1);
        assert_eq!(results.len(), 20, "Should have 20 root moves");
        let total: u64 = results.iter().map(|(_, c)| c).sum();
        assert_eq!(total, 20, "Total nodes from divide should match perft");
    }

    #[test]
    fn perft_divide_sums_match_perft() {
        init();
        let mut board = Board::new();
        let results = perft_divide(&mut board, 3);
        let total: u64 = results.iter().map(|(_, c)| c).sum();
        assert_eq!(total, 8902, "perft_divide sum at depth 3 should match perft(3)");
    }

    #[test]
    fn perft_depth_0_returns_1() {
        init();
        let mut board = Board::new();
        assert_eq!(perft(&mut board, 0), 1);
    }

    #[test]
    fn perft_checkmate_returns_0() {
        init();
        // Fool's mate position — checkmate, no legal moves
        let mut board = Board::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3").unwrap();
        assert_eq!(perft(&mut board, 1), 0);
    }
}
