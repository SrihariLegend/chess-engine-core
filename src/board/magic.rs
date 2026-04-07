// Magic bitboard tables and attack generation

use std::sync::OnceLock;

use super::Color;

// ─── Sliding Piece Magic Bitboard Tables ─────────────────────────────────────

/// A magic entry for one square: mask, magic multiplier, and shift amount.
pub struct MagicEntry {
    pub mask: u64,
    pub magic: u64,
    pub shift: u8,
}

/// Sliding piece attack tables, initialized once at startup.
struct SlidingTables {
    bishop_magics: Vec<MagicEntry>,
    rook_magics: Vec<MagicEntry>,
    bishop_attacks: Vec<Vec<u64>>,
    rook_attacks: Vec<Vec<u64>>,
}

static SLIDING_TABLES: OnceLock<SlidingTables> = OnceLock::new();

/// Initializes all attack tables (both non-sliding and sliding).
/// Call once at startup.
pub fn init_magic_tables() {
    init_attack_tables();
    SLIDING_TABLES.get_or_init(build_sliding_tables);
}

fn sliding() -> &'static SlidingTables {
    SLIDING_TABLES.get_or_init(build_sliding_tables)
}

/// Returns the attack bitboard for a bishop on `sq` with the given occupancy.
#[inline]
pub fn bishop_attacks(sq: u8, occupancy: u64) -> u64 {
    let tbl = sliding();
    let entry = &tbl.bishop_magics[sq as usize];
    let relevant = occupancy & entry.mask;
    let index = (relevant.wrapping_mul(entry.magic) >> entry.shift) as usize;
    tbl.bishop_attacks[sq as usize][index]
}

/// Returns the attack bitboard for a rook on `sq` with the given occupancy.
#[inline]
pub fn rook_attacks(sq: u8, occupancy: u64) -> u64 {
    let tbl = sliding();
    let entry = &tbl.rook_magics[sq as usize];
    let relevant = occupancy & entry.mask;
    let index = (relevant.wrapping_mul(entry.magic) >> entry.shift) as usize;
    tbl.rook_attacks[sq as usize][index]
}

/// Returns the attack bitboard for a queen on `sq` with the given occupancy.
/// Queen attacks = bishop attacks | rook attacks.
#[inline]
pub fn queen_attacks(sq: u8, occupancy: u64) -> u64 {
    bishop_attacks(sq, occupancy) | rook_attacks(sq, occupancy)
}

// ─── Reference Ray-Casting ───────────────────────────────────────────────────

/// Direction offsets for bishop rays: (rank_delta, file_delta).
const BISHOP_DIRS: [(i8, i8); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];

/// Direction offsets for rook rays: (rank_delta, file_delta).
const ROOK_DIRS: [(i8, i8); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// Reference ray-casting attack computation for a sliding piece.
/// Iterates along each direction from `sq`, stopping at the first blocker
/// (the blocker square IS included in the attack set).
pub fn ray_attacks(sq: u8, occupancy: u64, directions: &[(i8, i8)]) -> u64 {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let mut attacks = 0u64;

    for &(dr, df) in directions {
        let mut r = rank + dr;
        let mut f = file + df;
        while r >= 0 && r < 8 && f >= 0 && f < 8 {
            let bit = 1u64 << (r as u8 * 8 + f as u8);
            attacks |= bit;
            if occupancy & bit != 0 {
                break; // hit a blocker, stop this ray
            }
            r += dr;
            f += df;
        }
    }
    attacks
}

/// Reference bishop attacks via ray-casting.
pub fn bishop_attacks_ref(sq: u8, occupancy: u64) -> u64 {
    ray_attacks(sq, occupancy, &BISHOP_DIRS)
}

/// Reference rook attacks via ray-casting.
pub fn rook_attacks_ref(sq: u8, occupancy: u64) -> u64 {
    ray_attacks(sq, occupancy, &ROOK_DIRS)
}

// ─── Magic Table Construction ────────────────────────────────────────────────

/// Computes the relevant occupancy mask for a sliding piece on `sq`.
/// This includes all squares on the piece's rays EXCLUDING edge squares
/// (because edge squares don't affect whether the ray continues).
fn compute_mask(sq: u8, directions: &[(i8, i8)]) -> u64 {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let mut mask = 0u64;

    for &(dr, df) in directions {
        let mut r = rank + dr;
        let mut f = file + df;
        while r >= 0 && r < 8 && f >= 0 && f < 8 {
            // Exclude edge squares: if the NEXT step would go off-board, this is an edge
            let next_r = r + dr;
            let next_f = f + df;
            if next_r >= 0 && next_r < 8 && next_f >= 0 && next_f < 8 {
                mask |= 1u64 << (r as u8 * 8 + f as u8);
            }
            r = next_r;
            f = next_f;
        }
    }
    mask
}

/// Enumerates all subsets of a given mask using the Carry-Rippler trick.
fn enumerate_subsets(mask: u64) -> Vec<u64> {
    let mut subsets = Vec::new();
    let mut subset: u64 = 0;
    loop {
        subsets.push(subset);
        if subset == mask {
            break;
        }
        // Carry-Rippler: next subset
        subset = subset.wrapping_sub(mask) & mask;
    }
    subsets
}

/// Simple xorshift64 PRNG with fixed seed for reproducibility.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    /// Generate a sparse random u64 (good for magic number candidates).
    fn next_sparse(&mut self) -> u64 {
        self.next() & self.next() & self.next()
    }
}

/// Finds a magic number for a given square and piece type.
/// Returns (magic, shift, attack_table).
fn find_magic(
    sq: u8,
    mask: u64,
    directions: &[(i8, i8)],
    rng: &mut Rng,
) -> (u64, u8, Vec<u64>) {
    let bits = mask.count_ones() as u8;
    let shift = 64 - bits;
    let table_size = 1usize << bits;

    // Precompute all occupancy subsets and their reference attacks
    let subsets = enumerate_subsets(mask);
    let ref_attacks: Vec<u64> = subsets
        .iter()
        .map(|&occ| ray_attacks(sq, occ, directions))
        .collect();

    // Brute-force search for a magic number
    loop {
        let magic = rng.next_sparse();
        if magic == 0 {
            continue;
        }

        // Quick reject: the magic must map the mask to enough high bits
        if (mask.wrapping_mul(magic) & 0xFF00_0000_0000_0000).count_ones() < 6 {
            continue;
        }

        let mut table = vec![u64::MAX; table_size]; // MAX = unused sentinel
        let mut ok = true;

        for (i, &occ) in subsets.iter().enumerate() {
            let index = (occ.wrapping_mul(magic) >> shift) as usize;
            if table[index] == u64::MAX {
                table[index] = ref_attacks[i];
            } else if table[index] != ref_attacks[i] {
                ok = false;
                break;
            }
            // If table[index] == ref_attacks[i], it's a constructive collision (same attacks), OK
        }

        if ok {
            // Replace any remaining sentinel values with 0
            for entry in table.iter_mut() {
                if *entry == u64::MAX {
                    *entry = 0;
                }
            }
            return (magic, shift, table);
        }
    }
}

/// Builds all sliding piece magic tables.
fn build_sliding_tables() -> SlidingTables {
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);

    let mut bishop_magics = Vec::with_capacity(64);
    let mut rook_magics = Vec::with_capacity(64);
    let mut bishop_attacks_tbl = Vec::with_capacity(64);
    let mut rook_attacks_tbl = Vec::with_capacity(64);

    for sq in 0..64u8 {
        let mask = compute_mask(sq, &BISHOP_DIRS);
        let (magic, shift, table) = find_magic(sq, mask, &BISHOP_DIRS, &mut rng);
        bishop_magics.push(MagicEntry { mask, magic, shift });
        bishop_attacks_tbl.push(table);
    }

    for sq in 0..64u8 {
        let mask = compute_mask(sq, &ROOK_DIRS);
        let (magic, shift, table) = find_magic(sq, mask, &ROOK_DIRS, &mut rng);
        rook_magics.push(MagicEntry { mask, magic, shift });
        rook_attacks_tbl.push(table);
    }

    SlidingTables {
        bishop_magics,
        rook_magics,
        bishop_attacks: bishop_attacks_tbl,
        rook_attacks: rook_attacks_tbl,
    }
}

// ─── Non-Sliding Piece Attack Tables ─────────────────────────────────────────

struct AttackTables {
    knight: [u64; 64],
    king: [u64; 64],
    pawn: [[u64; 64]; 2], // [0] = White, [1] = Black
}

static ATTACK_TABLES: OnceLock<AttackTables> = OnceLock::new();

/// Initializes all precomputed attack tables. Call once at startup.
pub fn init_attack_tables() {
    ATTACK_TABLES.get_or_init(|| {
        let mut tables = AttackTables {
            knight: [0u64; 64],
            king: [0u64; 64],
            pawn: [[0u64; 64]; 2],
        };

        for sq in 0..64u8 {
            tables.knight[sq as usize] = compute_knight_attacks(sq);
            tables.king[sq as usize] = compute_king_attacks(sq);
            tables.pawn[0][sq as usize] = compute_pawn_attacks(sq, Color::White);
            tables.pawn[1][sq as usize] = compute_pawn_attacks(sq, Color::Black);
        }

        tables
    });
}

fn tables() -> &'static AttackTables {
    ATTACK_TABLES.get_or_init(|| {
        let mut tables = AttackTables {
            knight: [0u64; 64],
            king: [0u64; 64],
            pawn: [[0u64; 64]; 2],
        };

        for sq in 0..64u8 {
            tables.knight[sq as usize] = compute_knight_attacks(sq);
            tables.king[sq as usize] = compute_king_attacks(sq);
            tables.pawn[0][sq as usize] = compute_pawn_attacks(sq, Color::White);
            tables.pawn[1][sq as usize] = compute_pawn_attacks(sq, Color::Black);
        }

        tables
    })
}

// ─── Accessor Functions ──────────────────────────────────────────────────────

/// Returns the precomputed attack bitboard for a knight on the given square.
#[inline]
pub fn knight_attacks(sq: u8) -> u64 {
    tables().knight[sq as usize]
}

/// Returns the precomputed attack bitboard for a king on the given square.
#[inline]
pub fn king_attacks(sq: u8) -> u64 {
    tables().king[sq as usize]
}

/// Returns the precomputed attack bitboard for a pawn of the given color on the given square.
#[inline]
pub fn pawn_attacks(sq: u8, color: Color) -> u64 {
    tables().pawn[color.index()][sq as usize]
}

// ─── Computation Functions ───────────────────────────────────────────────────

/// Computes knight attacks from a given square.
/// Knight moves: (rank±1, file±2) and (rank±2, file±1).
fn compute_knight_attacks(sq: u8) -> u64 {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let offsets: [(i8, i8); 8] = [
        (-2, -1), (-2, 1),
        (-1, -2), (-1, 2),
        ( 1, -2), ( 1, 2),
        ( 2, -1), ( 2, 1),
    ];

    let mut attacks = 0u64;
    for (dr, df) in offsets {
        let r = rank + dr;
        let f = file + df;
        if r >= 0 && r < 8 && f >= 0 && f < 8 {
            attacks |= 1u64 << (r as u8 * 8 + f as u8);
        }
    }
    attacks
}

/// Computes king attacks from a given square.
/// King moves: all 8 adjacent squares.
fn compute_king_attacks(sq: u8) -> u64 {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let offsets: [(i8, i8); 8] = [
        (-1, -1), (-1, 0), (-1, 1),
        ( 0, -1),          ( 0, 1),
        ( 1, -1), ( 1, 0), ( 1, 1),
    ];

    let mut attacks = 0u64;
    for (dr, df) in offsets {
        let r = rank + dr;
        let f = file + df;
        if r >= 0 && r < 8 && f >= 0 && f < 8 {
            attacks |= 1u64 << (r as u8 * 8 + f as u8);
        }
    }
    attacks
}

/// Computes pawn attacks from a given square for the given color.
/// White pawns attack (rank+1, file-1) and (rank+1, file+1).
/// Black pawns attack (rank-1, file-1) and (rank-1, file+1).
fn compute_pawn_attacks(sq: u8, color: Color) -> u64 {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let dr: i8 = match color {
        Color::White => 1,
        Color::Black => -1,
    };

    let mut attacks = 0u64;
    let r = rank + dr;
    if r >= 0 && r < 8 {
        if file - 1 >= 0 {
            attacks |= 1u64 << (r as u8 * 8 + (file - 1) as u8);
        }
        if file + 1 < 8 {
            attacks |= 1u64 << (r as u8 * 8 + (file + 1) as u8);
        }
    }
    attacks
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: square index from algebraic-style (file, rank) where a1 = (0,0).
    fn sq(file: u8, rank: u8) -> u8 {
        rank * 8 + file
    }

    // ─── Non-Sliding Piece Tests ─────────────────────────────────────────────

    #[test]
    fn knight_on_e4_attacks_8_squares() {
        let attacks = knight_attacks(sq(4, 3));
        assert_eq!(attacks.count_ones(), 8, "Knight on e4 should attack 8 squares");
    }

    #[test]
    fn knight_on_a1_attacks_2_squares() {
        let attacks = knight_attacks(sq(0, 0));
        assert_eq!(attacks.count_ones(), 2, "Knight on a1 should attack 2 squares");
    }

    #[test]
    fn king_on_e4_attacks_8_squares() {
        let attacks = king_attacks(sq(4, 3));
        assert_eq!(attacks.count_ones(), 8, "King on e4 should attack 8 squares");
    }

    #[test]
    fn king_on_a1_attacks_3_squares() {
        let attacks = king_attacks(sq(0, 0));
        assert_eq!(attacks.count_ones(), 3, "King on a1 should attack 3 squares");
    }

    #[test]
    fn white_pawn_on_e4_attacks_d5_and_f5() {
        let attacks = pawn_attacks(sq(4, 3), Color::White);
        let d5 = 1u64 << sq(3, 4);
        let f5 = 1u64 << sq(5, 4);
        assert_eq!(attacks, d5 | f5, "White pawn on e4 should attack d5 and f5");
    }

    #[test]
    fn black_pawn_on_e5_attacks_d4_and_f4() {
        let attacks = pawn_attacks(sq(4, 4), Color::Black);
        let d4 = 1u64 << sq(3, 3);
        let f4 = 1u64 << sq(5, 3);
        assert_eq!(attacks, d4 | f4, "Black pawn on e5 should attack d4 and f4");
    }

    // ─── Sliding Piece Tests ─────────────────────────────────────────────────

    #[test]
    fn bishop_on_e4_empty_board() {
        // e4 = sq(4,3) = 28. On empty board, bishop attacks all 4 diagonals.
        // NE: f5,g6,h7 (3), NW: d5,c6,b7,a8 (4), SE: f3,g2,h1 (3), SW: d3,c2,b1,a0? d3,c2,b1 (3)
        // Total: 3+4+3+3 = 13
        let attacks = bishop_attacks(sq(4, 3), 0);
        assert_eq!(
            attacks.count_ones(),
            13,
            "Bishop on e4 empty board should attack 13 squares"
        );
        // Verify it matches reference
        assert_eq!(attacks, bishop_attacks_ref(sq(4, 3), 0));
    }

    #[test]
    fn rook_on_e4_empty_board() {
        // e4 = sq(4,3) = 28. On empty board, rook attacks entire rank and file minus itself.
        // Rank 3: a4..h4 minus e4 = 7, File e: e1..e8 minus e4 = 7. Total = 14
        let attacks = rook_attacks(sq(4, 3), 0);
        assert_eq!(
            attacks.count_ones(),
            14,
            "Rook on e4 empty board should attack 14 squares"
        );
        assert_eq!(attacks, rook_attacks_ref(sq(4, 3), 0));
    }

    #[test]
    fn queen_equals_bishop_or_rook() {
        let sq_e4 = sq(4, 3);
        let occ = 0u64;
        assert_eq!(
            queen_attacks(sq_e4, occ),
            bishop_attacks(sq_e4, occ) | rook_attacks(sq_e4, occ),
            "Queen attacks should equal bishop | rook attacks"
        );
    }

    #[test]
    fn rook_on_a1_blocked_at_a4() {
        // Rook on a1 (sq 0), blocker on a4 (sq 24).
        // Along file a: a2(8), a3(16), a4(24) — stops at a4 (included).
        // Along rank 1: b1(1), c1(2), d1(3), e1(4), f1(5), g1(6), h1(7) — 7 squares.
        // Total: 3 + 7 = 10
        let a1 = sq(0, 0);
        let a4 = sq(0, 3);
        let occ = 1u64 << a4;
        let attacks = rook_attacks(a1, occ);
        // a4 should be attacked (blocker is included)
        assert_ne!(attacks & (1u64 << a4), 0, "Rook should attack the blocker on a4");
        // a5 should NOT be attacked
        let a5 = sq(0, 4);
        assert_eq!(attacks & (1u64 << a5), 0, "Rook should not attack past blocker a4");
        assert_eq!(attacks, rook_attacks_ref(a1, occ));
    }

    #[test]
    fn bishop_on_a1_blocked_at_d4() {
        // Bishop on a1 (sq 0), blocker on d4 (sq 27).
        // Only diagonal from a1 goes NE: b2(9), c3(18), d4(27) — stops at d4.
        // Total: 3 squares
        let a1 = sq(0, 0);
        let d4 = sq(3, 3);
        let occ = 1u64 << d4;
        let attacks = bishop_attacks(a1, occ);
        // d4 should be attacked
        assert_ne!(attacks & (1u64 << d4), 0, "Bishop should attack the blocker on d4");
        // e5 should NOT be attacked
        let e5 = sq(4, 4);
        assert_eq!(attacks & (1u64 << e5), 0, "Bishop should not attack past blocker d4");
        assert_eq!(attacks.count_ones(), 3, "Bishop on a1 blocked at d4 should attack 3 squares");
        assert_eq!(attacks, bishop_attacks_ref(a1, occ));
    }

    #[test]
    fn queen_attacks_with_blockers() {
        // Verify queen = bishop | rook with some occupancy
        let sq_d4 = sq(3, 3);
        let occ = (1u64 << sq(3, 5)) | (1u64 << sq(5, 5)) | (1u64 << sq(1, 1));
        assert_eq!(
            queen_attacks(sq_d4, occ),
            bishop_attacks(sq_d4, occ) | rook_attacks(sq_d4, occ),
            "Queen attacks with blockers should equal bishop | rook"
        );
    }
}
