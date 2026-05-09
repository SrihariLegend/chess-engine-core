pub mod magic;

use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use std::sync::OnceLock;

// ─── Zobrist Hashing ─────────────────────────────────────────────────────────

/// Pseudorandom 64-bit keys for Zobrist hashing.
/// Total: 768 piece-square + 1 side-to-move + 4 castling + 8 en passant = 781 keys.
pub struct ZobristKeys {
    pub piece_square: [[[u64; 64]; 6]; 2], // [color][piece][square]
    pub side_to_move: u64,
    pub castling: [u64; 4], // WK, WQ, BK, BQ
    pub en_passant: [u64; 8], // per file
}

impl ZobristKeys {
    /// Initialize all Zobrist keys using a fixed-seed xorshift64 PRNG.
    fn init() -> Self {
        let mut state: u64 = 0x3A47_B2C1_D5E6_F708; // fixed seed

        let mut next = || -> u64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };

        let mut piece_square = [[[0u64; 64]; 6]; 2];
        for color in 0..2 {
            for piece in 0..6 {
                for sq in 0..64 {
                    piece_square[color][piece][sq] = next();
                }
            }
        }

        let side_to_move = next();

        let mut castling = [0u64; 4];
        for i in 0..4 {
            castling[i] = next();
        }

        let mut en_passant = [0u64; 8];
        for i in 0..8 {
            en_passant[i] = next();
        }

        ZobristKeys {
            piece_square,
            side_to_move,
            castling,
            en_passant,
        }
    }
}

/// Global Zobrist key set, initialized once on first access.
static ZOBRIST_KEYS: OnceLock<ZobristKeys> = OnceLock::new();

/// Returns a reference to the global Zobrist keys.
pub fn zobrist_keys() -> &'static ZobristKeys {
    ZOBRIST_KEYS.get_or_init(ZobristKeys::init)
}

// ─── Piece ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Piece {
    /// Returns the index of this piece type (0–5).
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }
}

// ─── Color ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    /// Returns the opposite color.
    #[inline]
    pub fn opposite(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    /// Returns the index of this color (0 for White, 1 for Black).
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }
}

// ─── MoveFlags ───────────────────────────────────────────────────────────────

/// Manual bitflags for move classification. No external crate.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MoveFlags(u8);

impl MoveFlags {
    pub const QUIET: MoveFlags = MoveFlags(0b0000_0000);
    pub const DOUBLE_PUSH: MoveFlags = MoveFlags(0b0000_0001);
    pub const KING_CASTLE: MoveFlags = MoveFlags(0b0000_0010);
    pub const QUEEN_CASTLE: MoveFlags = MoveFlags(0b0000_0100);
    pub const EN_PASSANT: MoveFlags = MoveFlags(0b0000_1000);
    pub const PROMOTION: MoveFlags = MoveFlags(0b0001_0000);

    #[inline]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[inline]
    pub const fn from_bits(bits: u8) -> Self {
        MoveFlags(bits)
    }

    #[inline]
    pub const fn contains(self, other: MoveFlags) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for MoveFlags {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        MoveFlags(self.0 | rhs.0)
    }
}

impl BitOrAssign for MoveFlags {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for MoveFlags {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        MoveFlags(self.0 & rhs.0)
    }
}

impl BitAndAssign for MoveFlags {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for MoveFlags {
    type Output = Self;
    #[inline]
    fn not(self) -> Self {
        MoveFlags(!self.0)
    }
}

impl fmt::Debug for MoveFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        let mut write_flag = |name: &str, f: &mut fmt::Formatter<'_>| -> fmt::Result {
            if !first {
                write!(f, " | ")?;
            }
            first = false;
            write!(f, "{}", name)
        };

        write!(f, "MoveFlags(")?;
        if self.is_empty() {
            write!(f, "QUIET")?;
        } else {
            if self.contains(MoveFlags::DOUBLE_PUSH) {
                write_flag("DOUBLE_PUSH", f)?;
            }
            if self.contains(MoveFlags::KING_CASTLE) {
                write_flag("KING_CASTLE", f)?;
            }
            if self.contains(MoveFlags::QUEEN_CASTLE) {
                write_flag("QUEEN_CASTLE", f)?;
            }
            if self.contains(MoveFlags::EN_PASSANT) {
                write_flag("EN_PASSANT", f)?;
            }
            if self.contains(MoveFlags::PROMOTION) {
                write_flag("PROMOTION", f)?;
            }
        }
        write!(f, ")")
    }
}

// ─── CastlingRights ──────────────────────────────────────────────────────────

/// Castling availability encoded as a 4-bit mask.
/// Bits: WK=0x1, WQ=0x2, BK=0x4, BQ=0x8
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CastlingRights(u8);

impl CastlingRights {
    pub const WK: u8 = 0x1;
    pub const WQ: u8 = 0x2;
    pub const BK: u8 = 0x4;
    pub const BQ: u8 = 0x8;
    pub const ALL: u8 = Self::WK | Self::WQ | Self::BK | Self::BQ;
    pub const NONE: u8 = 0;

    #[inline]
    pub const fn new(bits: u8) -> Self {
        CastlingRights(bits & Self::ALL)
    }

    #[inline]
    pub const fn empty() -> Self {
        CastlingRights(0)
    }

    #[inline]
    pub const fn all() -> Self {
        CastlingRights(Self::ALL)
    }

    #[inline]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Check if a specific right is set.
    #[inline]
    pub const fn has(self, right: u8) -> bool {
        (self.0 & right) != 0
    }

    /// Set a specific right.
    #[inline]
    pub fn set(&mut self, right: u8) {
        self.0 |= right & Self::ALL;
    }

    /// Clear a specific right.
    #[inline]
    pub fn clear(&mut self, right: u8) {
        self.0 &= !right;
    }

    /// Check if white can castle kingside.
    #[inline]
    pub const fn white_kingside(self) -> bool {
        self.has(Self::WK)
    }

    /// Check if white can castle queenside.
    #[inline]
    pub const fn white_queenside(self) -> bool {
        self.has(Self::WQ)
    }

    /// Check if black can castle kingside.
    #[inline]
    pub const fn black_kingside(self) -> bool {
        self.has(Self::BK)
    }

    /// Check if black can castle queenside.
    #[inline]
    pub const fn black_queenside(self) -> bool {
        self.has(Self::BQ)
    }

    /// Check if the given color can castle kingside.
    #[inline]
    pub const fn kingside(self, color: Color) -> bool {
        match color {
            Color::White => self.white_kingside(),
            Color::Black => self.black_kingside(),
        }
    }

    /// Check if the given color can castle queenside.
    #[inline]
    pub const fn queenside(self, color: Color) -> bool {
        match color {
            Color::White => self.white_queenside(),
            Color::Black => self.black_queenside(),
        }
    }
}

// ─── GamePhase ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamePhase {
    Opening,
    EarlyMiddlegame,
    LateMiddlegame,
    Endgame,
}

// ─── Move ────────────────────────────────────────────────────────────────────

/// Represents a single chess move. Designed to fit in 8 bytes.
///
/// Layout (8 bytes):
///   from: u8, to: u8, piece: u8 (Piece discriminant),
///   captured: u8 (0xFF = None, else Piece discriminant),
///   promotion: u8 (0xFF = None, else Piece discriminant),
///   flags: u8, padding: 2 bytes
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    pub piece: Piece,
    pub captured: Option<Piece>,
    pub promotion: Option<Piece>,
    pub flags: MoveFlags,
}

impl Move {
    #[inline]
    pub fn new(
        from: u8,
        to: u8,
        piece: Piece,
        captured: Option<Piece>,
        promotion: Option<Piece>,
        flags: MoveFlags,
    ) -> Self {
        Move {
            from,
            to,
            piece,
            captured,
            promotion,
            flags,
        }
    }

    /// A quiet (non-capture, non-special) move.
    #[inline]
    pub fn quiet(from: u8, to: u8, piece: Piece) -> Self {
        Move::new(from, to, piece, None, None, MoveFlags::QUIET)
    }

    /// Is this a capture move?
    #[inline]
    pub fn is_capture(&self) -> bool {
        self.captured.is_some()
    }

    /// Is this a promotion move?
    #[inline]
    pub fn is_promotion(&self) -> bool {
        self.flags.contains(MoveFlags::PROMOTION)
    }

    /// Is this a castling move?
    #[inline]
    pub fn is_castling(&self) -> bool {
        self.flags.contains(MoveFlags::KING_CASTLE) || self.flags.contains(MoveFlags::QUEEN_CASTLE)
    }

    /// Is this an en passant capture?
    #[inline]
    pub fn is_en_passant(&self) -> bool {
        self.flags.contains(MoveFlags::EN_PASSANT)
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file_from = (self.from % 8) as u8 + b'a';
        let rank_from = (self.from / 8) as u8 + b'1';
        let file_to = (self.to % 8) as u8 + b'a';
        let rank_to = (self.to / 8) as u8 + b'1';
        write!(
            f,
            "Move({}{}{}{}, {:?}",
            file_from as char,
            rank_from as char,
            file_to as char,
            rank_to as char,
            self.piece,
        )?;
        if let Some(cap) = self.captured {
            write!(f, ", cap={:?}", cap)?;
        }
        if let Some(promo) = self.promotion {
            write!(f, ", promo={:?}", promo)?;
        }
        if !self.flags.is_empty() {
            write!(f, ", {:?}", self.flags)?;
        }
        write!(f, ")")
    }
}

// ─── UndoInfo ────────────────────────────────────────────────────────────────

/// State saved before a move for the undo stack.
#[derive(Clone, Debug)]
pub struct UndoInfo {
    pub captured: Option<Piece>,
    pub castling: CastlingRights,
    pub en_passant: Option<u8>,
    pub halfmove_clock: u16,
    pub fullmove_number: u16,
    pub zobrist_hash: u64,
}

// ─── FenError ─────────────────────────────────────────────────────────────────

/// Errors that can occur when parsing a FEN string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FenError {
    InvalidFieldCount(usize),
    InvalidRank { rank: usize, reason: String },
    InvalidPiece(char),
    InvalidSideToMove(String),
    InvalidCastling(String),
    InvalidEnPassant(String),
    InvalidHalfmoveClock(String),
    InvalidFullmoveNumber(String),
}

impl fmt::Display for FenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FenError::InvalidFieldCount(n) => {
                write!(f, "expected 6 space-separated fields, got {}", n)
            }
            FenError::InvalidRank { rank, reason } => {
                write!(f, "invalid rank {}: {}", rank, reason)
            }
            FenError::InvalidPiece(ch) => {
                write!(f, "invalid piece character: '{}'", ch)
            }
            FenError::InvalidSideToMove(s) => {
                write!(f, "invalid side to move: '{}' (expected 'w' or 'b')", s)
            }
            FenError::InvalidCastling(s) => {
                write!(f, "invalid castling availability: '{}'", s)
            }
            FenError::InvalidEnPassant(s) => {
                write!(f, "invalid en passant square: '{}'", s)
            }
            FenError::InvalidHalfmoveClock(s) => {
                write!(f, "invalid halfmove clock: '{}'", s)
            }
            FenError::InvalidFullmoveNumber(s) => {
                write!(f, "invalid fullmove number: '{}'", s)
            }
        }
    }
}

// ─── Board ───────────────────────────────────────────────────────────────────

/// Core board state using bitboard representation.
///
/// Uses 12 piece bitboards (one per piece type per color), aggregate occupancy
/// bitboards, and standard chess state (side to move, castling, en passant, clocks).
///
/// Square mapping: a1=0, b1=1, ..., h1=7, a2=8, ..., h8=63
#[derive(Clone)]
pub struct Board {
    /// Piece bitboards indexed by [Color][Piece].
    pub pieces: [[u64; 6]; 2],
    /// Per-color occupancy bitboards.
    pub occupancy: [u64; 2],
    /// Combined occupancy of all pieces.
    pub all_occupancy: u64,
    /// Side to move.
    pub side_to_move: Color,
    /// Castling availability.
    pub castling: CastlingRights,
    /// En passant target square index (None if not available).
    pub en_passant: Option<u8>,
    /// Halfmove clock for the 50-move rule.
    pub halfmove_clock: u16,
    /// Fullmove number (starts at 1, incremented after Black's move).
    pub fullmove_number: u16,
    /// Zobrist hash of the current position.
    pub zobrist_hash: u64,
    /// Undo stack for unmake_move.
    history: Vec<UndoInfo>,
    /// Position hashes since last irreversible move (for repetition detection).
    position_history: Vec<u64>,
}

impl Board {
    /// Creates a new board set to the standard chess starting position.
    pub fn new() -> Self {
        let mut pieces = [[0u64; 6]; 2];

        // White pieces
        pieces[Color::White.index()][Piece::Pawn.index()] = 0x0000_0000_0000_FF00; // rank 2
        pieces[Color::White.index()][Piece::Knight.index()] = 0x0000_0000_0000_0042; // b1, g1
        pieces[Color::White.index()][Piece::Bishop.index()] = 0x0000_0000_0000_0024; // c1, f1
        pieces[Color::White.index()][Piece::Rook.index()] = 0x0000_0000_0000_0081; // a1, h1
        pieces[Color::White.index()][Piece::Queen.index()] = 0x0000_0000_0000_0008; // d1
        pieces[Color::White.index()][Piece::King.index()] = 0x0000_0000_0000_0010; // e1

        // Black pieces
        pieces[Color::Black.index()][Piece::Pawn.index()] = 0x00FF_0000_0000_0000; // rank 7
        pieces[Color::Black.index()][Piece::Knight.index()] = 0x4200_0000_0000_0000; // b8, g8
        pieces[Color::Black.index()][Piece::Bishop.index()] = 0x2400_0000_0000_0000; // c8, f8
        pieces[Color::Black.index()][Piece::Rook.index()] = 0x8100_0000_0000_0000; // a8, h8
        pieces[Color::Black.index()][Piece::Queen.index()] = 0x0800_0000_0000_0000; // d8
        pieces[Color::Black.index()][Piece::King.index()] = 0x1000_0000_0000_0000; // e8

        // Compute occupancy
        let white_occ = pieces[0].iter().fold(0u64, |acc, &bb| acc | bb);
        let black_occ = pieces[1].iter().fold(0u64, |acc, &bb| acc | bb);

        let mut board = Board {
            pieces,
            occupancy: [white_occ, black_occ],
            all_occupancy: white_occ | black_occ,
            side_to_move: Color::White,
            castling: CastlingRights::all(),
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            zobrist_hash: 0,
            history: Vec::new(),
            position_history: Vec::new(),
        };
        board.zobrist_hash = board.compute_zobrist_hash();
        board.position_history.push(board.zobrist_hash);
        board
    }

    /// Computes the full Zobrist hash from scratch for the current position.
    pub fn compute_zobrist_hash(&self) -> u64 {
        let keys = zobrist_keys();
        let mut hash = 0u64;

        // Piece-square keys
        for color in 0..2 {
            for piece in 0..6 {
                let mut bb = self.pieces[color][piece];
                while bb != 0 {
                    let sq = bb.trailing_zeros() as usize;
                    hash ^= keys.piece_square[color][piece][sq];
                    bb &= bb - 1; // clear lowest set bit
                }
            }
        }

        // Side to move
        if self.side_to_move == Color::Black {
            hash ^= keys.side_to_move;
        }

        // Castling rights
        if self.castling.has(CastlingRights::WK) {
            hash ^= keys.castling[0];
        }
        if self.castling.has(CastlingRights::WQ) {
            hash ^= keys.castling[1];
        }
        if self.castling.has(CastlingRights::BK) {
            hash ^= keys.castling[2];
        }
        if self.castling.has(CastlingRights::BQ) {
            hash ^= keys.castling[3];
        }

        // En passant file
        if let Some(ep_sq) = self.en_passant {
            let file = (ep_sq % 8) as usize;
            hash ^= keys.en_passant[file];
        }

        hash
    }

    /// Returns the game phase value for tapered evaluation interpolation.
    ///
    /// Phase is computed from remaining non-pawn, non-king material:
    /// - Knight = 1, Bishop = 1, Rook = 2, Queen = 4
    /// - Maximum phase = 24 (all minor/major pieces on board)
    /// - Returns value in range [0, 24] where 24 = opening, 0 = endgame
    pub fn game_phase(&self) -> i32 {
        let mut phase = 0i32;

        for color_idx in 0..2 {
            phase += self.pieces[color_idx][Piece::Knight.index()].count_ones() as i32; // 1 per knight
            phase += self.pieces[color_idx][Piece::Bishop.index()].count_ones() as i32; // 1 per bishop
            phase += (self.pieces[color_idx][Piece::Rook.index()].count_ones() as i32) * 2; // 2 per rook
            phase += (self.pieces[color_idx][Piece::Queen.index()].count_ones() as i32) * 4; // 4 per queen
        }

        // Clamp to [0, 24]
        phase.min(24)
    }

    /// Returns the piece and color on the given square, or None if empty.
    pub fn piece_at(&self, sq: u8) -> Option<(Color, Piece)> {
        let mask = 1u64 << sq;

        for color in [Color::White, Color::Black] {
            if self.occupancy[color.index()] & mask == 0 {
                continue;
            }
            for piece in [
                Piece::Pawn,
                Piece::Knight,
                Piece::Bishop,
                Piece::Rook,
                Piece::Queen,
                Piece::King,
            ] {
                if self.pieces[color.index()][piece.index()] & mask != 0 {
                    return Some((color, piece));
                }
            }
        }

        None
    }

    /// Parses a FEN string and returns a Board, or a descriptive error.
    ///
    /// FEN format: "<pieces> <side> <castling> <en_passant> <halfmove> <fullmove>"
    pub fn from_fen(fen: &str) -> Result<Self, FenError> {
        let fields: Vec<&str> = fen.split_whitespace().collect();
        if fields.len() != 6 {
            return Err(FenError::InvalidFieldCount(fields.len()));
        }

        // 1. Parse piece placement
        let mut pieces = [[0u64; 6]; 2];
        let ranks: Vec<&str> = fields[0].split('/').collect();
        if ranks.len() != 8 {
            return Err(FenError::InvalidRank {
                rank: 0,
                reason: format!("expected 8 ranks separated by '/', got {}", ranks.len()),
            });
        }

        for (rank_idx, rank_str) in ranks.iter().enumerate() {
            // FEN ranks go from rank 8 (index 0) down to rank 1 (index 7)
            let rank = 7 - rank_idx; // board rank: 7=rank8, 0=rank1
            let mut file = 0u8;

            for ch in rank_str.chars() {
                if file > 8 {
                    return Err(FenError::InvalidRank {
                        rank: 8 - rank_idx,
                        reason: "rank has more than 8 squares".to_string(),
                    });
                }
                if let Some(skip) = ch.to_digit(10) {
                    if skip < 1 || skip > 8 {
                        return Err(FenError::InvalidRank {
                            rank: 8 - rank_idx,
                            reason: format!("invalid digit '{}' in rank", ch),
                        });
                    }
                    file += skip as u8;
                } else {
                    if file >= 8 {
                        return Err(FenError::InvalidRank {
                            rank: 8 - rank_idx,
                            reason: "rank has more than 8 squares".to_string(),
                        });
                    }
                    let (color, piece) = match ch {
                        'P' => (Color::White, Piece::Pawn),
                        'N' => (Color::White, Piece::Knight),
                        'B' => (Color::White, Piece::Bishop),
                        'R' => (Color::White, Piece::Rook),
                        'Q' => (Color::White, Piece::Queen),
                        'K' => (Color::White, Piece::King),
                        'p' => (Color::Black, Piece::Pawn),
                        'n' => (Color::Black, Piece::Knight),
                        'b' => (Color::Black, Piece::Bishop),
                        'r' => (Color::Black, Piece::Rook),
                        'q' => (Color::Black, Piece::Queen),
                        'k' => (Color::Black, Piece::King),
                        _ => return Err(FenError::InvalidPiece(ch)),
                    };
                    let sq = rank as u8 * 8 + file;
                    pieces[color.index()][piece.index()] |= 1u64 << sq;
                    file += 1;
                }
            }

            if file != 8 {
                return Err(FenError::InvalidRank {
                    rank: 8 - rank_idx,
                    reason: format!("rank sums to {} squares, expected 8", file),
                });
            }
        }

        // 2. Parse side to move
        let side_to_move = match fields[1] {
            "w" => Color::White,
            "b" => Color::Black,
            other => return Err(FenError::InvalidSideToMove(other.to_string())),
        };

        // 3. Parse castling rights
        let castling = if fields[2] == "-" {
            CastlingRights::empty()
        } else {
            let mut rights = 0u8;
            for ch in fields[2].chars() {
                match ch {
                    'K' => rights |= CastlingRights::WK,
                    'Q' => rights |= CastlingRights::WQ,
                    'k' => rights |= CastlingRights::BK,
                    'q' => rights |= CastlingRights::BQ,
                    _ => return Err(FenError::InvalidCastling(fields[2].to_string())),
                }
            }
            CastlingRights::new(rights)
        };

        // 4. Parse en passant
        let en_passant = if fields[3] == "-" {
            None
        } else {
            let bytes = fields[3].as_bytes();
            if bytes.len() != 2 {
                return Err(FenError::InvalidEnPassant(fields[3].to_string()));
            }
            let file = bytes[0];
            let rank = bytes[1];
            if !(b'a'..=b'h').contains(&file) || !(b'1'..=b'8').contains(&rank) {
                return Err(FenError::InvalidEnPassant(fields[3].to_string()));
            }
            let sq = (rank - b'1') * 8 + (file - b'a');
            Some(sq)
        };

        // 5. Parse halfmove clock
        let halfmove_clock: u16 = fields[4]
            .parse()
            .map_err(|_| FenError::InvalidHalfmoveClock(fields[4].to_string()))?;

        // 6. Parse fullmove number
        let fullmove_number: u16 = fields[5]
            .parse()
            .map_err(|_| FenError::InvalidFullmoveNumber(fields[5].to_string()))?;
        if fullmove_number < 1 {
            return Err(FenError::InvalidFullmoveNumber(fields[5].to_string()));
        }

        // Compute occupancy
        let white_occ = pieces[0].iter().fold(0u64, |acc, &bb| acc | bb);
        let black_occ = pieces[1].iter().fold(0u64, |acc, &bb| acc | bb);

        let mut board = Board {
            pieces,
            occupancy: [white_occ, black_occ],
            all_occupancy: white_occ | black_occ,
            side_to_move,
            castling,
            en_passant,
            halfmove_clock,
            fullmove_number,
            zobrist_hash: 0,
            history: Vec::new(),
            position_history: Vec::new(),
        };
        board.zobrist_hash = board.compute_zobrist_hash();
        board.position_history.push(board.zobrist_hash);
        Ok(board)
    }

    /// Helper: recompute occupancy bitboards from piece bitboards.
    #[inline]
    fn update_occupancy(&mut self) {
        self.occupancy[0] = self.pieces[0].iter().fold(0u64, |acc, &bb| acc | bb);
        self.occupancy[1] = self.pieces[1].iter().fold(0u64, |acc, &bb| acc | bb);
        self.all_occupancy = self.occupancy[0] | self.occupancy[1];
    }

    /// Returns true if the given color's king is attacked by any opponent piece.
    pub fn is_in_check(&self, color: Color) -> bool {
        let king_bb = self.pieces[color.index()][Piece::King.index()];
        if king_bb == 0 {
            return false;
        }
        let king_sq = king_bb.trailing_zeros() as u8;
        let opp = color.opposite();
        let occ = self.all_occupancy;

        // Check opponent pawns
        if magic::pawn_attacks(king_sq, color) & self.pieces[opp.index()][Piece::Pawn.index()] != 0 {
            return true;
        }
        // Check opponent knights
        if magic::knight_attacks(king_sq) & self.pieces[opp.index()][Piece::Knight.index()] != 0 {
            return true;
        }
        // Check opponent bishops/queens (diagonal)
        if magic::bishop_attacks(king_sq, occ) & (self.pieces[opp.index()][Piece::Bishop.index()] | self.pieces[opp.index()][Piece::Queen.index()]) != 0 {
            return true;
        }
        // Check opponent rooks/queens (straight)
        if magic::rook_attacks(king_sq, occ) & (self.pieces[opp.index()][Piece::Rook.index()] | self.pieces[opp.index()][Piece::Queen.index()]) != 0 {
            return true;
        }
        // Check opponent king
        if magic::king_attacks(king_sq) & self.pieces[opp.index()][Piece::King.index()] != 0 {
            return true;
        }
        false
    }

    /// Returns true if the given square is attacked by any piece of the given color.
    pub fn is_square_attacked(&self, sq: u8, by_color: Color) -> bool {
        let occ = self.all_occupancy;
        let them = by_color.index();

        if magic::pawn_attacks(sq, by_color.opposite()) & self.pieces[them][Piece::Pawn.index()] != 0 {
            return true;
        }
        if magic::knight_attacks(sq) & self.pieces[them][Piece::Knight.index()] != 0 {
            return true;
        }
        if magic::bishop_attacks(sq, occ) & (self.pieces[them][Piece::Bishop.index()] | self.pieces[them][Piece::Queen.index()]) != 0 {
            return true;
        }
        if magic::rook_attacks(sq, occ) & (self.pieces[them][Piece::Rook.index()] | self.pieces[them][Piece::Queen.index()]) != 0 {
            return true;
        }
        if magic::king_attacks(sq) & self.pieces[them][Piece::King.index()] != 0 {
            return true;
        }
        false
    }

    /// Returns true if the current position is a repetition (has occurred before).
    pub fn is_repetition(&self) -> bool {
        self.position_history.iter().rev().skip(1)
            .any(|&h| h == self.zobrist_hash)
    }

    /// Returns true if this position has occurred before in the game (for TT safety).
    pub fn has_occurred_before(&self) -> bool {
        let count = self.position_history.iter().filter(|&&h| h == self.zobrist_hash).count();
        count > 1
    }

    /// Applies a move to the board, updating all state incrementally.
    ///
    /// Saves undo information to the history stack so the move can be reversed
    /// with `unmake_move`.
    pub fn make_move(&mut self, mv: Move) {
        let keys = zobrist_keys();
        let us = self.side_to_move.index();
        let them = self.side_to_move.opposite().index();

        // 1. Save undo info
        self.history.push(UndoInfo {
            captured: mv.captured,
            castling: self.castling,
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
            fullmove_number: self.fullmove_number,
            zobrist_hash: self.zobrist_hash,
        });

        let from = mv.from as usize;
        let to = mv.to as usize;
        let piece_idx = mv.piece.index();
        let from_bit = 1u64 << from;
        let to_bit = 1u64 << to;

        // 2. Remove moving piece from source
        self.pieces[us][piece_idx] ^= from_bit;
        self.zobrist_hash ^= keys.piece_square[us][piece_idx][from];

        // 3. Handle captures (non-en-passant)
        if let Some(cap) = mv.captured {
            if mv.flags.contains(MoveFlags::EN_PASSANT) {
                // En passant: captured pawn is on a different square
                let cap_sq = if self.side_to_move == Color::White {
                    mv.to - 8 // captured pawn is one rank below target
                } else {
                    mv.to + 8 // captured pawn is one rank above target
                } as usize;
                let cap_bit = 1u64 << cap_sq;
                self.pieces[them][Piece::Pawn.index()] ^= cap_bit;
                self.zobrist_hash ^= keys.piece_square[them][Piece::Pawn.index()][cap_sq];
            } else {
                // Normal capture: remove captured piece from target square
                self.pieces[them][cap.index()] ^= to_bit;
                self.zobrist_hash ^= keys.piece_square[them][cap.index()][to];
            }
        }

        // 4. Place piece on target (or promoted piece)
        if let Some(promo) = mv.promotion {
            self.pieces[us][promo.index()] ^= to_bit;
            self.zobrist_hash ^= keys.piece_square[us][promo.index()][to];
        } else {
            self.pieces[us][piece_idx] ^= to_bit;
            self.zobrist_hash ^= keys.piece_square[us][piece_idx][to];
        }

        // 5. Handle castling: move the rook
        if mv.flags.contains(MoveFlags::KING_CASTLE) {
            let (rook_from, rook_to) = if self.side_to_move == Color::White {
                (7usize, 5usize) // h1 -> f1
            } else {
                (63usize, 61usize) // h8 -> f8
            };
            self.pieces[us][Piece::Rook.index()] ^= (1u64 << rook_from) | (1u64 << rook_to);
            self.zobrist_hash ^= keys.piece_square[us][Piece::Rook.index()][rook_from];
            self.zobrist_hash ^= keys.piece_square[us][Piece::Rook.index()][rook_to];
        } else if mv.flags.contains(MoveFlags::QUEEN_CASTLE) {
            let (rook_from, rook_to) = if self.side_to_move == Color::White {
                (0usize, 3usize) // a1 -> d1
            } else {
                (56usize, 59usize) // a8 -> d8
            };
            self.pieces[us][Piece::Rook.index()] ^= (1u64 << rook_from) | (1u64 << rook_to);
            self.zobrist_hash ^= keys.piece_square[us][Piece::Rook.index()][rook_from];
            self.zobrist_hash ^= keys.piece_square[us][Piece::Rook.index()][rook_to];
        }

        // 6. Update castling rights
        let old_castling = self.castling;
        // King moves clear both rights for that side
        if mv.piece == Piece::King {
            if self.side_to_move == Color::White {
                self.castling.clear(CastlingRights::WK | CastlingRights::WQ);
            } else {
                self.castling.clear(CastlingRights::BK | CastlingRights::BQ);
            }
        }
        // Rook moves or captures clear the relevant right
        // Check source square (rook moved)
        if mv.from == 0 { self.castling.clear(CastlingRights::WQ); }
        if mv.from == 7 { self.castling.clear(CastlingRights::WK); }
        if mv.from == 56 { self.castling.clear(CastlingRights::BQ); }
        if mv.from == 63 { self.castling.clear(CastlingRights::BK); }
        // Check target square (rook captured)
        if mv.to == 0 { self.castling.clear(CastlingRights::WQ); }
        if mv.to == 7 { self.castling.clear(CastlingRights::WK); }
        if mv.to == 56 { self.castling.clear(CastlingRights::BQ); }
        if mv.to == 63 { self.castling.clear(CastlingRights::BK); }

        // Update Zobrist for castling rights change
        // XOR out old, XOR in new
        for i in 0..4u8 {
            let right = 1u8 << i;
            if old_castling.has(right) != self.castling.has(right) {
                self.zobrist_hash ^= keys.castling[i as usize];
            }
        }

        // 7. Update en passant
        // XOR out old en passant
        if let Some(ep_sq) = self.en_passant {
            self.zobrist_hash ^= keys.en_passant[(ep_sq % 8) as usize];
        }
        // Set new en passant if double pawn push
        if mv.flags.contains(MoveFlags::DOUBLE_PUSH) {
            let ep_sq = if self.side_to_move == Color::White {
                mv.from + 8 // en passant square is behind the pawn (rank 3)
            } else {
                mv.from - 8 // en passant square is behind the pawn (rank 6)
            };
            self.en_passant = Some(ep_sq);
            self.zobrist_hash ^= keys.en_passant[(ep_sq % 8) as usize];
        } else {
            self.en_passant = None;
        }

        // 8. Update halfmove clock and position history
        if mv.piece == Piece::Pawn || mv.captured.is_some() {
            self.halfmove_clock = 0;
            self.position_history.clear();
        } else {
            self.halfmove_clock += 1;
        }

        // 9. Update fullmove number (increment after Black's move)
        if self.side_to_move == Color::Black {
            self.fullmove_number += 1;
        }

        // 10. Toggle side to move
        self.side_to_move = self.side_to_move.opposite();
        self.zobrist_hash ^= keys.side_to_move;

        // 11. Recompute occupancy
        self.update_occupancy();

        // 12. Record position for repetition detection
        self.position_history.push(self.zobrist_hash);
    }

    /// Reverses the last move, restoring the board to its previous state.
    ///
    /// The move passed must be the same move that was last applied via `make_move`.
    pub fn unmake_move(&mut self, mv: Move) {
        let undo = self.history.pop().expect("unmake_move called with empty history");

        // Toggle side to move back (the side that made the move)
        self.side_to_move = self.side_to_move.opposite();
        let us = self.side_to_move.index();
        let them = self.side_to_move.opposite().index();

        let from = mv.from as usize;
        let to = mv.to as usize;
        let piece_idx = mv.piece.index();
        let from_bit = 1u64 << from;
        let to_bit = 1u64 << to;

        // Remove piece (or promoted piece) from target square
        if let Some(promo) = mv.promotion {
            self.pieces[us][promo.index()] ^= to_bit;
        } else {
            self.pieces[us][piece_idx] ^= to_bit;
        }

        // Place original piece back on source square
        self.pieces[us][piece_idx] ^= from_bit;

        // Restore captured piece
        if let Some(cap) = undo.captured {
            if mv.flags.contains(MoveFlags::EN_PASSANT) {
                let cap_sq = if self.side_to_move == Color::White {
                    mv.to - 8
                } else {
                    mv.to + 8
                } as usize;
                self.pieces[them][Piece::Pawn.index()] ^= 1u64 << cap_sq;
            } else {
                self.pieces[them][cap.index()] ^= to_bit;
            }
        }

        // Undo castling rook move
        if mv.flags.contains(MoveFlags::KING_CASTLE) {
            let (rook_from, rook_to) = if self.side_to_move == Color::White {
                (7usize, 5usize)
            } else {
                (63usize, 61usize)
            };
            self.pieces[us][Piece::Rook.index()] ^= (1u64 << rook_from) | (1u64 << rook_to);
        } else if mv.flags.contains(MoveFlags::QUEEN_CASTLE) {
            let (rook_from, rook_to) = if self.side_to_move == Color::White {
                (0usize, 3usize)
            } else {
                (56usize, 59usize)
            };
            self.pieces[us][Piece::Rook.index()] ^= (1u64 << rook_from) | (1u64 << rook_to);
        }

        // Restore state from undo info
        self.castling = undo.castling;
        self.en_passant = undo.en_passant;
        self.halfmove_clock = undo.halfmove_clock;
        self.fullmove_number = undo.fullmove_number;
        self.zobrist_hash = undo.zobrist_hash;

        // Pop position from repetition history
        self.position_history.pop();

        // Recompute occupancy
        self.update_occupancy();
    }

    /// Makes a null move (passes the turn without moving a piece).
    /// Used for null move pruning in the search.
    pub fn make_null_move(&mut self) {
        let keys = zobrist_keys();

        // Save undo info (no captured piece, no move)
        self.history.push(UndoInfo {
            captured: None,
            castling: self.castling,
            en_passant: self.en_passant,
            halfmove_clock: self.halfmove_clock,
            fullmove_number: self.fullmove_number,
            zobrist_hash: self.zobrist_hash,
        });

        // Clear en passant
        if let Some(ep_sq) = self.en_passant {
            self.zobrist_hash ^= keys.en_passant[(ep_sq % 8) as usize];
        }
        self.en_passant = None;

        // Toggle side to move
        self.side_to_move = self.side_to_move.opposite();
        self.zobrist_hash ^= keys.side_to_move;

        self.halfmove_clock += 1;
    }

    /// Unmakes a null move, restoring the previous state.
    pub fn unmake_null_move(&mut self) {
        let undo = self.history.pop().expect("unmake_null_move called with empty history");
        self.side_to_move = self.side_to_move.opposite();
        self.en_passant = undo.en_passant;
        self.halfmove_clock = undo.halfmove_clock;
        self.fullmove_number = undo.fullmove_number;
        self.zobrist_hash = undo.zobrist_hash;
    }

    /// Exports the current board state as a FEN string.
    pub fn to_fen(&self) -> String {
        let mut fen = String::with_capacity(80);

        // 1. Piece placement (rank 8 down to rank 1)
        for rank in (0..8).rev() {
            let mut empty = 0u8;
            for file in 0..8u8 {
                let sq = rank * 8 + file;
                if let Some((color, piece)) = self.piece_at(sq) {
                    if empty > 0 {
                        fen.push((b'0' + empty) as char);
                        empty = 0;
                    }
                    let ch = match (color, piece) {
                        (Color::White, Piece::Pawn) => 'P',
                        (Color::White, Piece::Knight) => 'N',
                        (Color::White, Piece::Bishop) => 'B',
                        (Color::White, Piece::Rook) => 'R',
                        (Color::White, Piece::Queen) => 'Q',
                        (Color::White, Piece::King) => 'K',
                        (Color::Black, Piece::Pawn) => 'p',
                        (Color::Black, Piece::Knight) => 'n',
                        (Color::Black, Piece::Bishop) => 'b',
                        (Color::Black, Piece::Rook) => 'r',
                        (Color::Black, Piece::Queen) => 'q',
                        (Color::Black, Piece::King) => 'k',
                    };
                    fen.push(ch);
                } else {
                    empty += 1;
                }
            }
            if empty > 0 {
                fen.push((b'0' + empty) as char);
            }
            if rank > 0 {
                fen.push('/');
            }
        }

        fen.push(' ');

        // 2. Side to move
        fen.push(match self.side_to_move {
            Color::White => 'w',
            Color::Black => 'b',
        });

        fen.push(' ');

        // 3. Castling rights
        if self.castling.bits() == 0 {
            fen.push('-');
        } else {
            if self.castling.white_kingside() {
                fen.push('K');
            }
            if self.castling.white_queenside() {
                fen.push('Q');
            }
            if self.castling.black_kingside() {
                fen.push('k');
            }
            if self.castling.black_queenside() {
                fen.push('q');
            }
        }

        fen.push(' ');

        // 4. En passant
        match self.en_passant {
            None => fen.push('-'),
            Some(sq) => {
                let file = (sq % 8) + b'a';
                let rank = (sq / 8) + b'1';
                fen.push(file as char);
                fen.push(rank as char);
            }
        }

        fen.push(' ');

        // 5. Halfmove clock
        fen.push_str(&self.halfmove_clock.to_string());

        fen.push(' ');

        // 6. Fullmove number
        fen.push_str(&self.fullmove_number.to_string());

        fen
    }
}

// ─── Compile-time size checks ────────────────────────────────────────────────

// Move should fit in 8 bytes for efficient copying.
const _: () = assert!(std::mem::size_of::<Move>() <= 8);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piece_index() {
        assert_eq!(Piece::Pawn.index(), 0);
        assert_eq!(Piece::Knight.index(), 1);
        assert_eq!(Piece::Bishop.index(), 2);
        assert_eq!(Piece::Rook.index(), 3);
        assert_eq!(Piece::Queen.index(), 4);
        assert_eq!(Piece::King.index(), 5);
    }

    #[test]
    fn color_opposite() {
        assert_eq!(Color::White.opposite(), Color::Black);
        assert_eq!(Color::Black.opposite(), Color::White);
    }

    #[test]
    fn color_index() {
        assert_eq!(Color::White.index(), 0);
        assert_eq!(Color::Black.index(), 1);
    }

    #[test]
    fn move_flags_bitwise() {
        let flags = MoveFlags::DOUBLE_PUSH | MoveFlags::PROMOTION;
        assert!(flags.contains(MoveFlags::DOUBLE_PUSH));
        assert!(flags.contains(MoveFlags::PROMOTION));
        assert!(!flags.contains(MoveFlags::EN_PASSANT));
        assert!(!flags.is_empty());
        assert!(MoveFlags::QUIET.is_empty());
    }

    #[test]
    fn move_flags_and() {
        let flags = MoveFlags::DOUBLE_PUSH | MoveFlags::PROMOTION;
        let masked = flags & MoveFlags::PROMOTION;
        assert!(masked.contains(MoveFlags::PROMOTION));
        assert!(!masked.contains(MoveFlags::DOUBLE_PUSH));
    }

    #[test]
    fn move_flags_not() {
        let flags = !MoveFlags::QUIET;
        assert_eq!(flags.bits(), 0xFF);
    }

    #[test]
    fn castling_rights_basics() {
        let mut cr = CastlingRights::all();
        assert!(cr.white_kingside());
        assert!(cr.white_queenside());
        assert!(cr.black_kingside());
        assert!(cr.black_queenside());

        cr.clear(CastlingRights::WK);
        assert!(!cr.white_kingside());
        assert!(cr.white_queenside());

        cr.clear(CastlingRights::BQ);
        assert!(!cr.black_queenside());
        assert!(cr.black_kingside());
    }

    #[test]
    fn castling_rights_color_helpers() {
        let cr = CastlingRights::new(CastlingRights::WK | CastlingRights::BQ);
        assert!(cr.kingside(Color::White));
        assert!(!cr.queenside(Color::White));
        assert!(!cr.kingside(Color::Black));
        assert!(cr.queenside(Color::Black));
    }

    #[test]
    fn castling_rights_empty() {
        let cr = CastlingRights::empty();
        assert_eq!(cr.bits(), 0);
        assert!(!cr.has(CastlingRights::WK));
    }

    #[test]
    fn castling_rights_set() {
        let mut cr = CastlingRights::empty();
        cr.set(CastlingRights::BK);
        assert!(cr.black_kingside());
        assert!(!cr.white_kingside());
    }

    #[test]
    fn move_quiet() {
        let mv = Move::quiet(12, 28, Piece::Pawn);
        assert_eq!(mv.from, 12);
        assert_eq!(mv.to, 28);
        assert_eq!(mv.piece, Piece::Pawn);
        assert!(!mv.is_capture());
        assert!(!mv.is_promotion());
        assert!(!mv.is_castling());
        assert!(!mv.is_en_passant());
    }

    #[test]
    fn move_capture() {
        let mv = Move::new(27, 36, Piece::Knight, Some(Piece::Pawn), None, MoveFlags::QUIET);
        assert!(mv.is_capture());
        assert_eq!(mv.captured, Some(Piece::Pawn));
    }

    #[test]
    fn move_promotion() {
        let mv = Move::new(
            48, 56, Piece::Pawn, None, Some(Piece::Queen),
            MoveFlags::PROMOTION,
        );
        assert!(mv.is_promotion());
        assert_eq!(mv.promotion, Some(Piece::Queen));
    }

    #[test]
    fn move_castling() {
        let mv = Move::new(4, 6, Piece::King, None, None, MoveFlags::KING_CASTLE);
        assert!(mv.is_castling());
    }

    #[test]
    fn move_en_passant() {
        let mv = Move::new(
            36, 45, Piece::Pawn, Some(Piece::Pawn), None,
            MoveFlags::EN_PASSANT,
        );
        assert!(mv.is_en_passant());
        assert!(mv.is_capture());
    }

    #[test]
    fn move_size_fits_in_8_bytes() {
        assert!(std::mem::size_of::<Move>() <= 8);
    }

    // ─── Board tests ─────────────────────────────────────────────────────

    #[test]
    fn board_new_white_pawns() {
        let board = Board::new();
        assert_eq!(board.pieces[Color::White.index()][Piece::Pawn.index()], 0x0000_0000_0000_FF00);
    }

    #[test]
    fn board_new_black_pawns() {
        let board = Board::new();
        assert_eq!(board.pieces[Color::Black.index()][Piece::Pawn.index()], 0x00FF_0000_0000_0000);
    }

    #[test]
    fn board_new_white_knights() {
        let board = Board::new();
        // b1 = square 1, g1 = square 6
        assert_eq!(board.pieces[Color::White.index()][Piece::Knight.index()], (1u64 << 1) | (1u64 << 6));
    }

    #[test]
    fn board_new_black_knights() {
        let board = Board::new();
        // b8 = square 57, g8 = square 62
        assert_eq!(board.pieces[Color::Black.index()][Piece::Knight.index()], (1u64 << 57) | (1u64 << 62));
    }

    #[test]
    fn board_new_white_bishops() {
        let board = Board::new();
        // c1 = square 2, f1 = square 5
        assert_eq!(board.pieces[Color::White.index()][Piece::Bishop.index()], (1u64 << 2) | (1u64 << 5));
    }

    #[test]
    fn board_new_black_bishops() {
        let board = Board::new();
        // c8 = square 58, f8 = square 61
        assert_eq!(board.pieces[Color::Black.index()][Piece::Bishop.index()], (1u64 << 58) | (1u64 << 61));
    }

    #[test]
    fn board_new_white_rooks() {
        let board = Board::new();
        // a1 = square 0, h1 = square 7
        assert_eq!(board.pieces[Color::White.index()][Piece::Rook.index()], (1u64 << 0) | (1u64 << 7));
    }

    #[test]
    fn board_new_black_rooks() {
        let board = Board::new();
        // a8 = square 56, h8 = square 63
        assert_eq!(board.pieces[Color::Black.index()][Piece::Rook.index()], (1u64 << 56) | (1u64 << 63));
    }

    #[test]
    fn board_new_white_queen() {
        let board = Board::new();
        // d1 = square 3
        assert_eq!(board.pieces[Color::White.index()][Piece::Queen.index()], 1u64 << 3);
    }

    #[test]
    fn board_new_black_queen() {
        let board = Board::new();
        // d8 = square 59
        assert_eq!(board.pieces[Color::Black.index()][Piece::Queen.index()], 1u64 << 59);
    }

    #[test]
    fn board_new_white_king() {
        let board = Board::new();
        // e1 = square 4
        assert_eq!(board.pieces[Color::White.index()][Piece::King.index()], 1u64 << 4);
    }

    #[test]
    fn board_new_black_king() {
        let board = Board::new();
        // e8 = square 60
        assert_eq!(board.pieces[Color::Black.index()][Piece::King.index()], 1u64 << 60);
    }

    #[test]
    fn board_new_occupancy() {
        let board = Board::new();
        // White occupies ranks 1-2 (squares 0-15)
        assert_eq!(board.occupancy[Color::White.index()], 0x0000_0000_0000_FFFF);
        // Black occupies ranks 7-8 (squares 48-63)
        assert_eq!(board.occupancy[Color::Black.index()], 0xFFFF_0000_0000_0000);
        // All occupancy is both combined
        assert_eq!(board.all_occupancy, 0xFFFF_0000_0000_FFFF);
    }

    #[test]
    fn board_new_state() {
        let board = Board::new();
        assert_eq!(board.side_to_move, Color::White);
        assert_eq!(board.castling, CastlingRights::all());
        assert_eq!(board.en_passant, None);
        assert_eq!(board.halfmove_clock, 0);
        assert_eq!(board.fullmove_number, 1);
        assert_ne!(board.zobrist_hash, 0);
    }

    #[test]
    fn board_new_piece_count() {
        let board = Board::new();
        // 16 white pieces, 16 black pieces
        assert_eq!(board.occupancy[Color::White.index()].count_ones(), 16);
        assert_eq!(board.occupancy[Color::Black.index()].count_ones(), 16);
        assert_eq!(board.all_occupancy.count_ones(), 32);
    }

    #[test]
    fn board_game_phase_starting_position() {
        let board = Board::new();
        // 2 knights(1 each) + 2 bishops(1 each) + 2 rooks(2 each) + 1 queen(4) per side
        // = (2 + 2 + 4 + 4) * 2 = 24
        assert_eq!(board.game_phase(), 24);
    }

    #[test]
    fn board_game_phase_empty_board() {
        let mut board = Board::new();
        // Clear all pieces except kings
        for color_idx in 0..2 {
            for piece_idx in 0..5 {
                // Pawn through Queen
                board.pieces[color_idx][piece_idx] = 0;
            }
        }
        // Phase should be 0 (only kings remain, kings don't count)
        assert_eq!(board.game_phase(), 0);
    }

    #[test]
    fn board_game_phase_one_queen_remaining() {
        let mut board = Board::new();
        // Clear all non-king, non-pawn pieces
        for color_idx in 0..2 {
            board.pieces[color_idx][Piece::Knight.index()] = 0;
            board.pieces[color_idx][Piece::Bishop.index()] = 0;
            board.pieces[color_idx][Piece::Rook.index()] = 0;
            board.pieces[color_idx][Piece::Queen.index()] = 0;
        }
        // Add one white queen back
        board.pieces[Color::White.index()][Piece::Queen.index()] = 1u64 << 3;
        assert_eq!(board.game_phase(), 4);
    }

    #[test]
    fn board_piece_at_starting_position() {
        let board = Board::new();
        // a1 = white rook
        assert_eq!(board.piece_at(0), Some((Color::White, Piece::Rook)));
        // e1 = white king
        assert_eq!(board.piece_at(4), Some((Color::White, Piece::King)));
        // d8 = black queen (square 59)
        assert_eq!(board.piece_at(59), Some((Color::Black, Piece::Queen)));
        // e2 = white pawn (square 12)
        assert_eq!(board.piece_at(12), Some((Color::White, Piece::Pawn)));
        // e4 = empty (square 28)
        assert_eq!(board.piece_at(28), None);
        // h8 = black rook (square 63)
        assert_eq!(board.piece_at(63), Some((Color::Black, Piece::Rook)));
    }

    // ─── FEN parsing tests ───────────────────────────────────────────────

    const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[test]
    fn fen_parse_starting_position() {
        let board = Board::from_fen(START_FEN).unwrap();
        let expected = Board::new();
        assert_eq!(board.pieces, expected.pieces);
        assert_eq!(board.occupancy, expected.occupancy);
        assert_eq!(board.all_occupancy, expected.all_occupancy);
        assert_eq!(board.side_to_move, expected.side_to_move);
        assert_eq!(board.castling, expected.castling);
        assert_eq!(board.en_passant, expected.en_passant);
        assert_eq!(board.halfmove_clock, expected.halfmove_clock);
        assert_eq!(board.fullmove_number, expected.fullmove_number);
    }

    #[test]
    fn fen_parse_midgame_position() {
        // Kiwipete position
        let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        let board = Board::from_fen(fen).unwrap();
        assert_eq!(board.side_to_move, Color::White);
        assert_eq!(board.castling, CastlingRights::all());
        assert_eq!(board.en_passant, None);
        assert_eq!(board.halfmove_clock, 0);
        assert_eq!(board.fullmove_number, 1);
        // Check a few specific pieces
        // e5 = square 36, should be white knight
        assert_eq!(board.piece_at(36), Some((Color::White, Piece::Knight)));
        // d5 = square 35, should be white pawn
        assert_eq!(board.piece_at(35), Some((Color::White, Piece::Pawn)));
        // e7 = square 52, should be black queen
        assert_eq!(board.piece_at(52), Some((Color::Black, Piece::Queen)));
    }

    #[test]
    fn fen_parse_with_en_passant() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let board = Board::from_fen(fen).unwrap();
        assert_eq!(board.side_to_move, Color::Black);
        // e3 = file 4, rank 2 => square = 2*8 + 4 = 20
        assert_eq!(board.en_passant, Some(20));
    }

    #[test]
    fn fen_parse_partial_castling() {
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w Kq - 0 1";
        let board = Board::from_fen(fen).unwrap();
        assert!(board.castling.white_kingside());
        assert!(!board.castling.white_queenside());
        assert!(!board.castling.black_kingside());
        assert!(board.castling.black_queenside());
    }

    #[test]
    fn fen_parse_no_castling() {
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b - - 5 20";
        let board = Board::from_fen(fen).unwrap();
        assert_eq!(board.castling, CastlingRights::empty());
        assert_eq!(board.side_to_move, Color::Black);
        assert_eq!(board.halfmove_clock, 5);
        assert_eq!(board.fullmove_number, 20);
    }

    #[test]
    fn fen_round_trip_starting_position() {
        let board = Board::new();
        let fen = board.to_fen();
        assert_eq!(fen, START_FEN);
    }

    #[test]
    fn fen_round_trip_midgame() {
        let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        let board = Board::from_fen(fen).unwrap();
        let exported = board.to_fen();
        assert_eq!(exported, fen);
    }

    #[test]
    fn fen_round_trip_with_en_passant() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let board = Board::from_fen(fen).unwrap();
        let exported = board.to_fen();
        assert_eq!(exported, fen);
    }

    #[test]
    fn fen_error_wrong_field_count() {
        let result = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq");
        assert!(matches!(result, Err(FenError::InvalidFieldCount(3))));
    }

    #[test]
    fn fen_error_too_many_fields() {
        let result = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 extra");
        assert!(matches!(result, Err(FenError::InvalidFieldCount(7))));
    }

    #[test]
    fn fen_error_invalid_piece_char() {
        let result = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPXPPP/RNBQKBNR w KQkq - 0 1");
        assert!(matches!(result, Err(FenError::InvalidPiece('X'))));
    }

    #[test]
    fn fen_error_invalid_side_to_move() {
        let result = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR x KQkq - 0 1");
        assert!(matches!(result, Err(FenError::InvalidSideToMove(_))));
    }

    #[test]
    fn fen_error_invalid_castling() {
        let result = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQxq - 0 1");
        assert!(matches!(result, Err(FenError::InvalidCastling(_))));
    }

    #[test]
    fn fen_error_invalid_en_passant() {
        let result = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq z9 0 1");
        assert!(matches!(result, Err(FenError::InvalidEnPassant(_))));
    }

    #[test]
    fn fen_error_invalid_halfmove_clock() {
        let result = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - abc 1");
        assert!(matches!(result, Err(FenError::InvalidHalfmoveClock(_))));
    }

    #[test]
    fn fen_error_invalid_fullmove_number() {
        let result = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 xyz");
        assert!(matches!(result, Err(FenError::InvalidFullmoveNumber(_))));
    }

    #[test]
    fn fen_error_fullmove_zero() {
        let result = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 0");
        assert!(matches!(result, Err(FenError::InvalidFullmoveNumber(_))));
    }

    #[test]
    fn fen_error_rank_too_long() {
        let result = Board::from_fen("rnbqkbnrr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert!(matches!(result, Err(FenError::InvalidRank { .. })));
    }

    #[test]
    fn fen_error_rank_too_short() {
        let result = Board::from_fen("rnbqkbn/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert!(matches!(result, Err(FenError::InvalidRank { .. })));
    }

    #[test]
    fn fen_error_wrong_rank_count() {
        let result = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert!(matches!(result, Err(FenError::InvalidRank { .. })));
    }

    // ─── Zobrist hashing tests ───────────────────────────────────────────

    #[test]
    fn zobrist_starting_position_nonzero() {
        let board = Board::new();
        assert_ne!(board.zobrist_hash, 0);
    }

    #[test]
    fn zobrist_identical_positions_same_hash() {
        let board1 = Board::new();
        let board2 = Board::new();
        assert_eq!(board1.zobrist_hash, board2.zobrist_hash);
    }

    #[test]
    fn zobrist_fen_matches_new() {
        let board_new = Board::new();
        let board_fen =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(board_new.zobrist_hash, board_fen.zobrist_hash);
    }

    #[test]
    fn zobrist_different_positions_different_hash() {
        let board1 = Board::new();
        // After 1.e4
        let board2 =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1")
                .unwrap();
        assert_ne!(board1.zobrist_hash, board2.zobrist_hash);
    }

    #[test]
    fn zobrist_side_to_move_matters() {
        // Same piece placement, different side to move
        let board_w =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let board_b =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
        assert_ne!(board_w.zobrist_hash, board_b.zobrist_hash);
    }

    #[test]
    fn zobrist_castling_rights_matter() {
        let board_all =
            Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        let board_none =
            Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w - - 0 1").unwrap();
        assert_ne!(board_all.zobrist_hash, board_none.zobrist_hash);
    }

    #[test]
    fn zobrist_en_passant_matters() {
        let board_no_ep =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1")
                .unwrap();
        let board_ep =
            Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1")
                .unwrap();
        assert_ne!(board_no_ep.zobrist_hash, board_ep.zobrist_hash);
    }

    #[test]
    fn zobrist_compute_matches_stored() {
        let board = Board::new();
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn zobrist_keys_are_deterministic() {
        let keys1 = zobrist_keys();
        let keys2 = zobrist_keys();
        // Same reference from OnceLock
        assert_eq!(
            keys1.piece_square[0][0][0],
            keys2.piece_square[0][0][0]
        );
        assert_eq!(keys1.side_to_move, keys2.side_to_move);
    }

    // ─── Make/Unmake move tests ──────────────────────────────────────────

    /// Helper: snapshot all board state for comparison.
    fn board_snapshot(b: &Board) -> (
        [[u64; 6]; 2], [u64; 2], u64, Color, CastlingRights, Option<u8>, u16, u16, u64,
    ) {
        (
            b.pieces, b.occupancy, b.all_occupancy,
            b.side_to_move, b.castling, b.en_passant,
            b.halfmove_clock, b.fullmove_number, b.zobrist_hash,
        )
    }

    #[test]
    fn make_move_simple_pawn_push() {
        // e2-e3: pawn from sq 12 to sq 20
        let mut board = Board::new();
        let mv = Move::quiet(12, 20, Piece::Pawn);
        board.make_move(mv);

        // Pawn should be on e3, not e2
        let white_pawns = board.pieces[Color::White.index()][Piece::Pawn.index()];
        assert_eq!(white_pawns & (1u64 << 12), 0, "pawn should be gone from e2");
        assert_ne!(white_pawns & (1u64 << 20), 0, "pawn should be on e3");
        assert_eq!(board.side_to_move, Color::Black);
        assert_eq!(board.en_passant, None);
        assert_eq!(board.halfmove_clock, 0); // pawn move resets clock
        assert_eq!(board.fullmove_number, 1); // not incremented until after Black
        // Zobrist should match full recomputation
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn make_move_double_pawn_push() {
        // e2-e4: pawn from sq 12 to sq 28, sets en passant on e3 (sq 20)
        let mut board = Board::new();
        let mv = Move::new(12, 28, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH);
        board.make_move(mv);

        let white_pawns = board.pieces[Color::White.index()][Piece::Pawn.index()];
        assert_eq!(white_pawns & (1u64 << 12), 0);
        assert_ne!(white_pawns & (1u64 << 28), 0);
        assert_eq!(board.en_passant, Some(20)); // e3
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn make_move_pawn_capture() {
        // Set up: white pawn on e4 (28), black pawn on d5 (35)
        let fen = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2";
        let mut board = Board::from_fen(fen).unwrap();
        let mv = Move::new(28, 35, Piece::Pawn, Some(Piece::Pawn), None, MoveFlags::QUIET);
        board.make_move(mv);

        let wp = board.pieces[Color::White.index()][Piece::Pawn.index()];
        let bp = board.pieces[Color::Black.index()][Piece::Pawn.index()];
        assert_ne!(wp & (1u64 << 35), 0, "white pawn on d5");
        assert_eq!(bp & (1u64 << 35), 0, "black pawn removed from d5");
        assert_eq!(board.halfmove_clock, 0); // capture resets
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn make_move_kingside_castle_white() {
        // Position where white can castle kingside
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        let mv = Move::new(4, 6, Piece::King, None, None, MoveFlags::KING_CASTLE);
        board.make_move(mv);

        // King on g1 (6), rook on f1 (5)
        assert_ne!(board.pieces[0][Piece::King.index()] & (1u64 << 6), 0);
        assert_ne!(board.pieces[0][Piece::Rook.index()] & (1u64 << 5), 0);
        // King gone from e1, rook gone from h1
        assert_eq!(board.pieces[0][Piece::King.index()] & (1u64 << 4), 0);
        assert_eq!(board.pieces[0][Piece::Rook.index()] & (1u64 << 7), 0);
        // White castling rights cleared
        assert!(!board.castling.white_kingside());
        assert!(!board.castling.white_queenside());
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn make_move_queenside_castle_white() {
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        let mv = Move::new(4, 2, Piece::King, None, None, MoveFlags::QUEEN_CASTLE);
        board.make_move(mv);

        // King on c1 (2), rook on d1 (3)
        assert_ne!(board.pieces[0][Piece::King.index()] & (1u64 << 2), 0);
        assert_ne!(board.pieces[0][Piece::Rook.index()] & (1u64 << 3), 0);
        assert_eq!(board.pieces[0][Piece::King.index()] & (1u64 << 4), 0);
        assert_eq!(board.pieces[0][Piece::Rook.index()] & (1u64 << 0), 0);
        assert!(!board.castling.white_kingside());
        assert!(!board.castling.white_queenside());
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn make_move_kingside_castle_black() {
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        let mv = Move::new(60, 62, Piece::King, None, None, MoveFlags::KING_CASTLE);
        board.make_move(mv);

        assert_ne!(board.pieces[1][Piece::King.index()] & (1u64 << 62), 0);
        assert_ne!(board.pieces[1][Piece::Rook.index()] & (1u64 << 61), 0);
        assert!(!board.castling.black_kingside());
        assert!(!board.castling.black_queenside());
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn make_move_queenside_castle_black() {
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        let mv = Move::new(60, 58, Piece::King, None, None, MoveFlags::QUEEN_CASTLE);
        board.make_move(mv);

        assert_ne!(board.pieces[1][Piece::King.index()] & (1u64 << 58), 0);
        assert_ne!(board.pieces[1][Piece::Rook.index()] & (1u64 << 59), 0);
        assert!(!board.castling.black_kingside());
        assert!(!board.castling.black_queenside());
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn make_move_en_passant_white() {
        // White pawn on e5 (36), black pawn on d5 (35), en passant target d6 (43)
        let fen = "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3";
        let mut board = Board::from_fen(fen).unwrap();
        let mv = Move::new(
            36, 43, Piece::Pawn, Some(Piece::Pawn), None, MoveFlags::EN_PASSANT,
        );
        board.make_move(mv);

        let wp = board.pieces[0][Piece::Pawn.index()];
        let bp = board.pieces[1][Piece::Pawn.index()];
        assert_ne!(wp & (1u64 << 43), 0, "white pawn on d6");
        assert_eq!(wp & (1u64 << 36), 0, "white pawn gone from e5");
        assert_eq!(bp & (1u64 << 35), 0, "black pawn removed from d5 (ep capture)");
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn make_move_en_passant_black() {
        // Black pawn on d4 (27), white pawn on e4 (28), en passant target e3 (20)
        let fen = "rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 3";
        let mut board = Board::from_fen(fen).unwrap();
        let mv = Move::new(
            27, 20, Piece::Pawn, Some(Piece::Pawn), None, MoveFlags::EN_PASSANT,
        );
        board.make_move(mv);

        let bp = board.pieces[1][Piece::Pawn.index()];
        let wp = board.pieces[0][Piece::Pawn.index()];
        assert_ne!(bp & (1u64 << 20), 0, "black pawn on e3");
        assert_eq!(bp & (1u64 << 27), 0, "black pawn gone from d4");
        assert_eq!(wp & (1u64 << 28), 0, "white pawn removed from e4 (ep capture)");
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn make_move_promotion() {
        // White pawn on a7 (48) promotes to queen on a8 (56)
        let fen = "4k3/P7/8/8/8/8/8/4K3 w - - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        let mv = Move::new(
            48, 56, Piece::Pawn, None, Some(Piece::Queen), MoveFlags::PROMOTION,
        );
        board.make_move(mv);

        let wp = board.pieces[0][Piece::Pawn.index()];
        let wq = board.pieces[0][Piece::Queen.index()];
        assert_eq!(wp & (1u64 << 48), 0, "pawn gone from a7");
        assert_eq!(wp & (1u64 << 56), 0, "no pawn on a8");
        assert_ne!(wq & (1u64 << 56), 0, "queen on a8");
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn make_move_promotion_capture() {
        // White pawn on a7 (48) captures black rook on b8 (57) and promotes to queen
        let fen = "1r2k3/P7/8/8/8/8/8/4K3 w - - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        let mv = Move::new(
            48, 57, Piece::Pawn, Some(Piece::Rook), Some(Piece::Queen), MoveFlags::PROMOTION,
        );
        board.make_move(mv);

        assert_eq!(board.pieces[0][Piece::Pawn.index()] & (1u64 << 57), 0);
        assert_ne!(board.pieces[0][Piece::Queen.index()] & (1u64 << 57), 0);
        assert_eq!(board.pieces[1][Piece::Rook.index()] & (1u64 << 57), 0);
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn make_unmake_restores_simple_pawn_push() {
        let mut board = Board::new();
        let snap = board_snapshot(&board);
        let mv = Move::quiet(12, 20, Piece::Pawn);
        board.make_move(mv);
        board.unmake_move(mv);
        assert_eq!(board_snapshot(&board), snap);
    }

    #[test]
    fn make_unmake_restores_double_pawn_push() {
        let mut board = Board::new();
        let snap = board_snapshot(&board);
        let mv = Move::new(12, 28, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH);
        board.make_move(mv);
        board.unmake_move(mv);
        assert_eq!(board_snapshot(&board), snap);
    }

    #[test]
    fn make_unmake_restores_capture() {
        let fen = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2";
        let mut board = Board::from_fen(fen).unwrap();
        let snap = board_snapshot(&board);
        let mv = Move::new(28, 35, Piece::Pawn, Some(Piece::Pawn), None, MoveFlags::QUIET);
        board.make_move(mv);
        board.unmake_move(mv);
        assert_eq!(board_snapshot(&board), snap);
    }

    #[test]
    fn make_unmake_restores_kingside_castle() {
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        let snap = board_snapshot(&board);
        let mv = Move::new(4, 6, Piece::King, None, None, MoveFlags::KING_CASTLE);
        board.make_move(mv);
        board.unmake_move(mv);
        assert_eq!(board_snapshot(&board), snap);
    }

    #[test]
    fn make_unmake_restores_queenside_castle() {
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        let snap = board_snapshot(&board);
        let mv = Move::new(4, 2, Piece::King, None, None, MoveFlags::QUEEN_CASTLE);
        board.make_move(mv);
        board.unmake_move(mv);
        assert_eq!(board_snapshot(&board), snap);
    }

    #[test]
    fn make_unmake_restores_en_passant() {
        let fen = "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3";
        let mut board = Board::from_fen(fen).unwrap();
        let snap = board_snapshot(&board);
        let mv = Move::new(
            36, 43, Piece::Pawn, Some(Piece::Pawn), None, MoveFlags::EN_PASSANT,
        );
        board.make_move(mv);
        board.unmake_move(mv);
        assert_eq!(board_snapshot(&board), snap);
    }

    #[test]
    fn make_unmake_restores_promotion() {
        let fen = "4k3/P7/8/8/8/8/8/4K3 w - - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        let snap = board_snapshot(&board);
        let mv = Move::new(
            48, 56, Piece::Pawn, None, Some(Piece::Queen), MoveFlags::PROMOTION,
        );
        board.make_move(mv);
        board.unmake_move(mv);
        assert_eq!(board_snapshot(&board), snap);
    }

    #[test]
    fn make_unmake_restores_promotion_capture() {
        let fen = "1r2k3/P7/8/8/8/8/8/4K3 w - - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        let snap = board_snapshot(&board);
        let mv = Move::new(
            48, 57, Piece::Pawn, Some(Piece::Rook), Some(Piece::Queen), MoveFlags::PROMOTION,
        );
        board.make_move(mv);
        board.unmake_move(mv);
        assert_eq!(board_snapshot(&board), snap);
    }

    #[test]
    fn make_move_fullmove_increments_after_black() {
        let mut board = Board::new();
        // White move: e2-e4
        let mv_w = Move::new(12, 28, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH);
        board.make_move(mv_w);
        assert_eq!(board.fullmove_number, 1); // still 1 after white

        // Black move: e7-e5
        let mv_b = Move::new(52, 36, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH);
        board.make_move(mv_b);
        assert_eq!(board.fullmove_number, 2); // incremented after black
    }

    #[test]
    fn make_move_halfmove_clock_increments_on_quiet() {
        // Knight move: Nb1-c3 (sq 1 -> sq 18)
        let mut board = Board::new();
        let mv = Move::quiet(1, 18, Piece::Knight);
        board.make_move(mv);
        assert_eq!(board.halfmove_clock, 1);
    }

    #[test]
    fn make_move_rook_move_clears_castling() {
        let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1";
        let mut board = Board::from_fen(fen).unwrap();
        // Move white rook from h1 (7) to g1 (6)
        let mv = Move::quiet(7, 6, Piece::Rook);
        board.make_move(mv);
        assert!(!board.castling.white_kingside(), "WK should be cleared");
        assert!(board.castling.white_queenside(), "WQ should remain");
        assert_eq!(board.zobrist_hash, board.compute_zobrist_hash());
    }

    #[test]
    fn make_unmake_multiple_moves_sequence() {
        let mut board = Board::new();
        let snap = board_snapshot(&board);

        // 1. e2-e4
        let mv1 = Move::new(12, 28, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH);
        board.make_move(mv1);
        // 2. e7-e5
        let mv2 = Move::new(52, 36, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH);
        board.make_move(mv2);
        // 3. Ng1-f3 (sq 6 -> sq 21)
        let mv3 = Move::quiet(6, 21, Piece::Knight);
        board.make_move(mv3);

        // Unmake in reverse
        board.unmake_move(mv3);
        board.unmake_move(mv2);
        board.unmake_move(mv1);

        assert_eq!(board_snapshot(&board), snap);
    }
}
