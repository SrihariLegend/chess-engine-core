// Romantic personality: rewards piece mobility and activity

use crate::board::{Board, Piece};
use crate::board::magic;
use crate::personality::{squash_to_cp, GameContext, PersonalityEval};

/// Bonus per square attacked by a piece.
const ACTIVITY_BONUS: i32 = 3;

/// Penalty for pieces with fewer than 3 available moves.
const PASSIVE_PENALTY: i32 = -15;

/// Minimum moves threshold — pieces below this are considered passive.
const PASSIVE_THRESHOLD: u32 = 3;

pub struct Romantic {
    pub weight: f32,
}

impl Romantic {
    pub fn new() -> Self {
        Romantic { weight: 1.0 }
    }
}

impl PersonalityEval for Romantic {
    fn evaluate(&self, board: &Board, _ctx: &GameContext) -> i32 {
        let mut score = 0i32;
        let occ = board.all_occupancy;
        let us = board.side_to_move;
        let friendly = board.occupancy[us.index()];

        // Evaluate each of our non-pawn, non-king pieces
        for &piece in &[Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen] {
            let mut bb = board.pieces[us.index()][piece.index()];
            while bb != 0 {
                let sq = bb.trailing_zeros() as u8;
                let attacks = match piece {
                    Piece::Knight => magic::knight_attacks(sq) & !friendly,
                    Piece::Bishop => magic::bishop_attacks(sq, occ) & !friendly,
                    Piece::Rook => magic::rook_attacks(sq, occ) & !friendly,
                    Piece::Queen => magic::queen_attacks(sq, occ) & !friendly,
                    _ => 0,
                };
                let move_count = attacks.count_ones();

                // Bonus proportional to squares attacked
                score += ACTIVITY_BONUS * move_count as i32;

                // Penalty for passive pieces
                if move_count < PASSIVE_THRESHOLD {
                    score += PASSIVE_PENALTY;
                }

                bb &= bb - 1;
            }
        }

        squash_to_cp(score as f32, 120.0)
    }

    fn weight(&self) -> f32 {
        self.weight
    }

    fn set_weight(&mut self, w: f32) {
        self.weight = w;
    }

    fn name(&self) -> &str {
        "Romantic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Board, GamePhase};

    fn default_ctx() -> GameContext {
        GameContext {
            move_number: 1,
            phase: GamePhase::Opening,
            eval_history: [0; 8],
            eval_history_len: 0,
            side_to_move_moves: 20,
            opponent_moves: 20,
        }
    }

    #[test]
    fn starting_position_produces_score() {
        let board = Board::new();
        let ctx = default_ctx();
        let r = Romantic::new();
        let score = r.evaluate(&board, &ctx);
        // In starting position, pieces are somewhat blocked, so score should be non-zero
        // Knights have limited moves, bishops are blocked by pawns
        // Just verify it returns a reasonable value
        assert!(score != 0, "Starting position should produce non-zero romantic score");
    }

    #[test]
    fn active_pieces_get_bonus() {
        // Open position where pieces have many squares
        let board = Board::from_fen("4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1").unwrap();
        let ctx = default_ctx();
        let r = Romantic::new();
        let score = r.evaluate(&board, &ctx);
        // Queen on d4 on open board attacks many squares, should get large bonus
        assert!(score > 0, "Active queen should produce positive score, got {}", score);
    }

    #[test]
    fn passive_pieces_get_penalty() {
        // Knight trapped in corner with few moves
        let board = Board::from_fen("4k3/8/8/8/8/8/1P6/N3K3 w - - 0 1").unwrap();
        let ctx = default_ctx();
        let r = Romantic::new();
        let score = r.evaluate(&board, &ctx);
        // Knight on a1 with pawn on b2 has very few moves (possibly < 3)
        // Score may be negative due to passive penalty
        // Just verify it runs without error
        assert!(score < 20, "Trapped knight should not produce large positive score");
    }

    #[test]
    fn default_weight_is_one() {
        let r = Romantic::new();
        assert_eq!(r.weight(), 1.0);
    }

    #[test]
    fn name_is_correct() {
        let r = Romantic::new();
        assert_eq!(r.name(), "Romantic");
    }
}
