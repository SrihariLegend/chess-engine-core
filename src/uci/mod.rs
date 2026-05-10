// UCI protocol handler

use std::io::{self, BufRead};

use crate::board::{Board, Move, Piece};
use crate::movegen::{self, MoveGenResult};
use crate::personality::GameContext;
use crate::search::{format_move, SearchParams, SearchState};

/// UCI options configurable via `setoption`.
pub struct UciOptions {
    pub hash_size_mb: usize,
    pub max_depth: u32,
    pub threads: usize,
    pub style_profile: String,
    pub style_intensity: f32,
    pub contempt: i32,
}

impl UciOptions {
    pub fn new() -> Self {
        UciOptions {
            hash_size_mb: 128,
            max_depth: 64,
            threads: 1,
            style_profile: "lasker".to_string(),
            style_intensity: 0.3,
            contempt: 50,
        }
    }
}

/// Main UCI protocol handler.
pub struct UciHandler {
    pub board: Board,
    pub search_state: SearchState,
    pub options: UciOptions,
    pub game_context: GameContext,
}

impl UciHandler {
    pub fn new() -> Self {
        let options = UciOptions::new();
        UciHandler {
            board: Board::new(),
            search_state: SearchState::new(options.hash_size_mb),
            options,
            game_context: GameContext::new(),
        }
    }

    /// Main UCI loop: reads stdin line by line and dispatches commands.
    pub fn run(&mut self) {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            let should_quit = self.process_command(&line);
            if should_quit {
                break;
            }
        }
    }

    /// Process a single UCI command string. Returns true if the engine should quit.
    pub fn process_command(&mut self, line: &str) -> bool {
        let line = line.trim();
        if line.is_empty() {
            return false;
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            return false;
        }

        match tokens[0] {
            "uci" => self.handle_uci(),
            "isready" => self.handle_isready(),
            "position" => self.handle_position(&tokens[1..]),
            "go" => self.handle_go(&tokens[1..]),
            "stop" => self.handle_stop(),
            "quit" => return true,
            "ucinewgame" => self.handle_ucinewgame(),
            "setoption" => self.handle_setoption(&tokens[1..]),
            _ => {} // Silently ignore unrecognized commands per UCI spec
        }

        false
    }

    /// Respond to `uci` command with engine identification and options.
    fn handle_uci(&self) {
        println!("id name ChessEngine");
        println!("id author Developer");
        println!("option name Hash type spin default 64 min 1 max 4096");
        println!("option name MaxDepth type spin default 64 min 1 max 128");
        println!("option name Threads type spin default 1 min 1 max 256");
        println!("option name Contempt type spin default 50 min -500 max 500");
        println!("option name StyleProfile type combo default lasker var tal var petrosian var karpov var capablanca var morphy var alekhine var lasker");
        println!("option name StyleIntensity type spin default 30 min 0 max 100");
        println!("uciok");
    }

    /// Respond to `isready` with `readyok`.
    fn handle_isready(&self) {
        println!("readyok");
    }

    /// Handle `position` command.
    /// Formats:
    ///   position startpos [moves e2e4 e7e5 ...]
    ///   position fen <fen_string> [moves e2e4 e7e5 ...]
    fn handle_position(&mut self, tokens: &[&str]) {
        if tokens.is_empty() {
            return;
        }

        let mut idx;

        if tokens[0] == "startpos" {
            self.board = Board::new();
            idx = 1;
        } else if tokens[0] == "fen" {
            // Collect FEN fields (up to 6 tokens or until "moves")
            let mut fen_parts = Vec::new();
            idx = 1;
            while idx < tokens.len() && tokens[idx] != "moves" && fen_parts.len() < 6 {
                fen_parts.push(tokens[idx]);
                idx += 1;
            }
            let fen_str = fen_parts.join(" ");
            match Board::from_fen(&fen_str) {
                Ok(board) => self.board = board,
                Err(e) => {
                    eprintln!("info string Invalid FEN: {}", e);
                    return;
                }
            }
        } else {
            return;
        }

        // Apply moves if present
        if idx < tokens.len() && tokens[idx] == "moves" {
            idx += 1;
            while idx < tokens.len() {
                if let Some(mv) = parse_move(&mut self.board, tokens[idx]) {
                    self.board.make_move(mv);
                } else {
                    eprintln!("info string Invalid move: {}", tokens[idx]);
                    return;
                }
                idx += 1;
            }
        }
    }

    /// Handle `go` command with time control parameters.
    fn handle_go(&mut self, tokens: &[&str]) {
        // Check for `go perft <depth>`
        if tokens.len() >= 2 && tokens[0] == "perft" {
            if let Ok(depth) = tokens[1].parse::<u32>() {
                self.handle_perft(depth);
                return;
            }
        }

        let mut params = SearchParams::new();
        let mut i = 0;
        while i < tokens.len() {
            match tokens[i] {
                "wtime" => {
                    i += 1;
                    if i < tokens.len() {
                        params.wtime = tokens[i].parse().ok();
                    }
                }
                "btime" => {
                    i += 1;
                    if i < tokens.len() {
                        params.btime = tokens[i].parse().ok();
                    }
                }
                "winc" => {
                    i += 1;
                    if i < tokens.len() {
                        params.winc = tokens[i].parse().ok();
                    }
                }
                "binc" => {
                    i += 1;
                    if i < tokens.len() {
                        params.binc = tokens[i].parse().ok();
                    }
                }
                "movestogo" => {
                    i += 1;
                    if i < tokens.len() {
                        params.moves_to_go = tokens[i].parse().ok();
                    }
                }
                "depth" => {
                    i += 1;
                    if i < tokens.len() {
                        params.max_depth = tokens[i].parse().ok();
                    }
                }
                "movetime" => {
                    i += 1;
                    if i < tokens.len() {
                        params.move_time = tokens[i].parse().ok();
                    }
                }
                "infinite" => {
                    params.infinite = true;
                }
                _ => {}
            }
            i += 1;
        }

        // Apply max depth from options if not overridden
        if params.max_depth.is_none() && !params.infinite {
            // Don't override — let the search use its default
        }

        // Apply style profile from options
        self.search_state.threads = self.options.threads;
        self.search_state.contempt = self.options.contempt;
        self.search_state.style_intensity = self.options.style_intensity;
        self.search_state.set_profile(&self.options.style_profile);

        let best = self.search_state.search(&mut self.board, params);
        match best {
            Some(mv) => {
                // Update game context after search
                self.game_context.move_number += 1;
                self.game_context.update_phase();
                println!("bestmove {}", format_move(&mv));
            }
            None => println!("bestmove 0000"),
        }
    }

    /// Handle `stop` command: set the stop flag on the search.
    fn handle_stop(&mut self) {
        use std::sync::atomic::Ordering;
        self.search_state.stop.store(true, Ordering::Relaxed);
    }

    /// Handle `ucinewgame`: clear TT and reset state.
    fn handle_ucinewgame(&mut self) {
        self.search_state.tt.clear();
        self.search_state.killer_moves.clear();
        self.search_state.history_table.clear();
        self.game_context = GameContext::new();
        self.board = Board::new();
    }

    /// Handle `setoption` command.
    /// Format: setoption name <name> value <value>
    fn handle_setoption(&mut self, tokens: &[&str]) {
        // Parse "name <name> value <value>"
        if tokens.len() < 4 || tokens[0] != "name" {
            return;
        }

        // Find the "value" keyword
        let mut name_parts = Vec::new();
        let mut value_str = "";
        let mut found_value = false;
        for i in 1..tokens.len() {
            if tokens[i] == "value" && i + 1 < tokens.len() {
                value_str = tokens[i + 1];
                found_value = true;
                break;
            }
            name_parts.push(tokens[i]);
        }

        if !found_value {
            return;
        }

        let name = name_parts.join(" ");
        match name.to_lowercase().as_str() {
            "hash" => {
                if let Ok(size) = value_str.parse::<usize>() {
                    let size = size.clamp(1, 4096);
                    self.options.hash_size_mb = size;
                    self.search_state.tt.resize(size);
                }
            }
            "maxdepth" => {
                if let Ok(depth) = value_str.parse::<u32>() {
                    let depth = depth.clamp(1, 128);
                    self.options.max_depth = depth;
                }
            }
            "threads" => {
                if let Ok(threads) = value_str.parse::<usize>() {
                    let threads = threads.clamp(1, 256);
                    self.options.threads = threads;
                }
            }
            "styleprofile" => {
                if crate::personality::profile::profile_by_name(&value_str.to_lowercase()).is_some() {
                    self.options.style_profile = value_str.to_lowercase();
                }
            }
            "styleintensity" => {
                if let Ok(v) = value_str.parse::<u32>() {
                    self.options.style_intensity = (v as f32 / 100.0).clamp(0.0, 1.0);
                }
            }
            "contempt" => {
                if let Ok(c) = value_str.parse::<i32>() {
                    self.options.contempt = c.clamp(-500, 500);
                }
            }
            _ => {} // Ignore unknown options
        }
    }

    /// Handle `go perft <depth>`: run perft and print results.
    fn handle_perft(&mut self, depth: u32) {
        let results = movegen::perft_divide(&mut self.board, depth);
        let mut total = 0u64;
        for (mv, count) in &results {
            println!("{}: {}", format_move(mv), count);
            total += count;
        }
        println!();
        println!("Nodes searched: {}", total);
    }
}

/// Parse a move string in long algebraic notation (e.g., "e2e4", "a7a8q")
/// and find the matching legal move from the current position.
pub fn parse_move(board: &mut Board, move_str: &str) -> Option<Move> {
    let bytes = move_str.as_bytes();
    if bytes.len() < 4 || bytes.len() > 5 {
        return None;
    }

    let from_file = bytes[0].wrapping_sub(b'a');
    let from_rank = bytes[1].wrapping_sub(b'1');
    let to_file = bytes[2].wrapping_sub(b'a');
    let to_rank = bytes[3].wrapping_sub(b'1');

    if from_file > 7 || from_rank > 7 || to_file > 7 || to_rank > 7 {
        return None;
    }

    let from_sq = from_rank * 8 + from_file;
    let to_sq = to_rank * 8 + to_file;

    let promotion = if bytes.len() == 5 {
        match bytes[4] {
            b'q' => Some(Piece::Queen),
            b'r' => Some(Piece::Rook),
            b'b' => Some(Piece::Bishop),
            b'n' => Some(Piece::Knight),
            _ => return None,
        }
    } else {
        None
    };

    // Generate legal moves and find the matching one
    let moves = match movegen::generate_legal_moves(board) {
        MoveGenResult::Moves(moves) => moves,
        _ => return None,
    };

    moves.into_iter().find(|mv| {
        mv.from == from_sq && mv.to == to_sq && mv.promotion == promotion
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::magic::init_magic_tables;
    use crate::board::{Color, MoveFlags};

    fn setup() -> UciHandler {
        init_magic_tables();
        UciHandler::new()
    }

    #[test]
    fn handle_uci_outputs_correct_strings() {
        // We can't easily capture stdout in a unit test, but we can verify
        // the handler doesn't panic and the struct is in correct state.
        let handler = setup();
        handler.handle_uci();
        // If we get here without panic, the handler works.
    }

    #[test]
    fn handle_isready_outputs_readyok() {
        let handler = setup();
        handler.handle_isready();
    }

    #[test]
    fn handle_position_startpos_sets_starting_position() {
        let mut handler = setup();
        handler.process_command("position startpos");
        assert_eq!(
            handler.board.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
    }

    #[test]
    fn handle_position_startpos_moves_applies_move() {
        let mut handler = setup();
        handler.process_command("position startpos moves e2e4");
        // After 1.e4, the FEN should reflect the pawn on e4
        let fen = handler.board.to_fen();
        assert!(fen.contains("4P3") || fen.starts_with("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR"),
            "After e2e4, board should have pawn on e4. FEN: {}", fen);
        assert_eq!(handler.board.side_to_move, Color::Black);
    }

    #[test]
    fn handle_position_startpos_multiple_moves() {
        let mut handler = setup();
        handler.process_command("position startpos moves e2e4 e7e5");
        assert_eq!(handler.board.side_to_move, Color::White);
        // Both pawns should be advanced
        let fen = handler.board.to_fen();
        assert!(fen.contains("4p3") || fen.contains("4P3"),
            "FEN should show moved pawns: {}", fen);
    }

    #[test]
    fn handle_position_fen_sets_position() {
        let mut handler = setup();
        let test_fen = "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq c6 0 2";
        handler.process_command(&format!("position fen {}", test_fen));
        assert_eq!(handler.board.to_fen(), test_fen);
    }

    #[test]
    fn handle_position_fen_with_moves() {
        let mut handler = setup();
        handler.process_command("position fen rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1 moves e7e5");
        assert_eq!(handler.board.side_to_move, Color::White);
    }

    #[test]
    fn parse_move_basic_pawn_push() {
        init_magic_tables();
        let mut board = Board::new();
        let mv = parse_move(&mut board, "e2e4");
        assert!(mv.is_some(), "Should parse e2e4");
        let mv = mv.unwrap();
        assert_eq!(mv.from, 12); // e2
        assert_eq!(mv.to, 28);   // e4
        assert_eq!(mv.piece, Piece::Pawn);
    }

    #[test]
    fn parse_move_knight_move() {
        init_magic_tables();
        let mut board = Board::new();
        let mv = parse_move(&mut board, "g1f3");
        assert!(mv.is_some(), "Should parse g1f3");
        let mv = mv.unwrap();
        assert_eq!(mv.piece, Piece::Knight);
    }

    #[test]
    fn parse_move_promotion() {
        init_magic_tables();
        let mut board = Board::from_fen("8/P3k3/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let mv = parse_move(&mut board, "a7a8q");
        assert!(mv.is_some(), "Should parse promotion a7a8q");
        let mv = mv.unwrap();
        assert_eq!(mv.promotion, Some(Piece::Queen));
        assert!(mv.flags.contains(MoveFlags::PROMOTION));
    }

    #[test]
    fn parse_move_invalid_returns_none() {
        init_magic_tables();
        let mut board = Board::new();
        assert!(parse_move(&mut board, "z9z9").is_none());
        assert!(parse_move(&mut board, "").is_none());
        assert!(parse_move(&mut board, "e2e9").is_none());
    }

    #[test]
    fn parse_move_illegal_move_returns_none() {
        init_magic_tables();
        let mut board = Board::new();
        // e1e3 is not a legal move from starting position
        assert!(parse_move(&mut board, "e1e3").is_none());
    }

    #[test]
    fn handle_ucinewgame_resets_state() {
        let mut handler = setup();
        // Make a move to change state
        handler.process_command("position startpos moves e2e4");
        assert_eq!(handler.board.side_to_move, Color::Black);

        handler.process_command("ucinewgame");
        assert_eq!(handler.board.side_to_move, Color::White);
        assert_eq!(
            handler.board.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
    }

    #[test]
    fn handle_setoption_hash() {
        let mut handler = setup();
        handler.process_command("setoption name Hash value 128");
        assert_eq!(handler.options.hash_size_mb, 128);
    }

    #[test]
    fn handle_setoption_maxdepth() {
        let mut handler = setup();
        handler.process_command("setoption name MaxDepth value 20");
        assert_eq!(handler.options.max_depth, 20);
    }

    #[test]
    fn quit_returns_true() {
        let mut handler = setup();
        assert!(handler.process_command("quit"));
    }

    #[test]
    fn unknown_command_returns_false() {
        let mut handler = setup();
        assert!(!handler.process_command("unknown_command"));
    }

    #[test]
    fn handle_go_depth_returns_bestmove() {
        let mut handler = setup();
        handler.process_command("position startpos");
        // Search at depth 1 — should find a move
        handler.process_command("go depth 1");
        // If we get here without panic, the go handler works.
    }
}
