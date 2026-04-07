use chess_engine_core::board;
use chess_engine_core::movegen;
use chess_engine_core::uci;

fn main() {
    // Initialize magic bitboard tables at startup
    board::magic::init_magic_tables();

    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 && args[1] == "--perft" {
        // Perft mode: --perft <depth> [fen]
        let depth: u32 = args[2].parse().expect("Invalid depth");
        let fen = if args.len() >= 4 {
            args[3..].join(" ")
        } else {
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string()
        };

        let mut board = board::Board::from_fen(&fen).expect("Invalid FEN");
        let nodes = movegen::perft(&mut board, depth);
        println!("Perft({}) = {}", depth, nodes);
    } else {
        // Default: UCI mode
        let mut handler = uci::UciHandler::new();
        handler.run();
    }
}
